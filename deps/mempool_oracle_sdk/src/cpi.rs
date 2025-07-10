use arch_program::program_error::ProgramError;
use arch_program::utxo::UtxoMeta;
use arch_program::{account::AccountInfo, instruction::Instruction, pubkey::Pubkey};
use arch_program::{
    account::AccountMeta,
    program::{get_return_data, invoke},
};
use saturn_collections::generic::fixed_list::FixedList;

use crate::errors::ErrorCode;
use crate::{ReturnedMempoolEntry, UpdateMempoolEntriesInstruction};

#[derive(Clone, Copy, Debug, Default)]
pub struct MempoolInfo {
    pub total_fee: u64,
    pub total_size: u64,
}

#[derive(Clone, Debug, Default)]
pub enum TxStatus {
    Pending(MempoolInfo),
    #[default]
    Confirmed,
}

#[derive(Debug, Clone, Default)]
pub struct AccountMempoolInfo {
    pub ancestors_count: u16,
    pub descendants_count: u16,
}

#[derive(Debug)]
pub struct MempoolData<const MAX_UTXOS: usize, const MAX_ACCOUNTS: usize> {
    // Up to N user-provided UTXOs
    utxo_mempool_info: [Option<([u8; 32], MempoolInfo)>; MAX_UTXOS],
    accounts_utxo_mempool_info: [AccountMempoolInfo; MAX_ACCOUNTS],
}

impl<const MAX_UTXOS: usize, const MAX_ACCOUNTS: usize> MempoolData<MAX_UTXOS, MAX_ACCOUNTS> {
    pub fn new(
        utxo_mempool_info: [Option<([u8; 32], MempoolInfo)>; MAX_UTXOS],
        accounts_utxo_mempool_info: [AccountMempoolInfo; MAX_ACCOUNTS],
    ) -> Self {
        Self {
            utxo_mempool_info,
            accounts_utxo_mempool_info,
        }
    }

    pub fn get_utxo_status(&self, txid: [u8; 32]) -> TxStatus {
        // Linear search is efficient for small collections and cache-friendly
        for (stored_txid, info) in self.utxo_mempool_info.iter().flatten() {
            if *stored_txid == txid {
                return TxStatus::Pending(*info);
            }
        }
        TxStatus::Confirmed
    }

    pub fn get_mempool_info_for_accounts(&self, n_accounts: usize) -> &[AccountMempoolInfo] {
        &self.accounts_utxo_mempool_info.split_at(n_accounts).0
    }
}

impl<const MAX_UTXOS: usize, const MAX_ACCOUNTS: usize> Default
    for MempoolData<MAX_UTXOS, MAX_ACCOUNTS>
{
    fn default() -> Self {
        MempoolData::new(
            [None; MAX_UTXOS],
            std::array::from_fn(|_| AccountMempoolInfo::default()),
        )
    }
}

pub fn exec_mempool_cpi<'a>(
    mempool_info_program_id: &Pubkey,
    mempool_info_pdas: &[AccountInfo<'a>],
    txids: Vec<[u8; 32]>,
) -> Result<Vec<Option<ReturnedMempoolEntry>>, ErrorCode> {
    let data = UpdateMempoolEntriesInstruction::GetEntries(txids);

    // Pre-allocate the accounts vector with exact capacity to avoid reallocations
    let mut accounts = Vec::with_capacity(mempool_info_pdas.len());
    for account in mempool_info_pdas.iter() {
        accounts.push(AccountMeta::new_readonly(*account.key, false));
    }

    invoke(
        &Instruction {
            program_id: *mempool_info_program_id,
            accounts,
            data: borsh::to_vec(&data).unwrap(),
        },
        mempool_info_pdas,
    )
    .map_err(|_| ErrorCode::MempoolCPIError)?;

    if let Some((_, map_data)) = get_return_data() {
        let return_map: Vec<Option<ReturnedMempoolEntry>> =
            borsh::from_slice(&map_data).map_err(|_| ErrorCode::InvalidMempoolData)?;
        Ok(return_map)
    } else {
        Err(ErrorCode::MempoolCPIError)
    }
}

pub fn get_mempool_data<'a, T, const MAX_UTXOS: usize, const MAX_ACCOUNTS: usize>(
    mempool_info_program_id: &'a Pubkey,
    mempool_info_pdas: &'a [AccountInfo<'static>],
    user_utxos: &[T],
    accounts: Option<&'a [AccountInfo<'static>]>,
) -> Result<MempoolData<MAX_UTXOS, MAX_ACCOUNTS>, ProgramError>
where
    T: AsRef<UtxoMeta>,
{
    // Pre-calculate capacity to avoid reallocation
    let account_count = accounts.map_or(0, |accounts| accounts.len());
    let capacity = user_utxos.len() + account_count;

    let mut txids = Vec::with_capacity(capacity);

    // Track user txids and their indices in the unified txids list
    let mut user_txids_with_indices = FixedList::<([u8; 32], usize), MAX_UTXOS>::new();

    // Process user UTXOs and build txid mapping without HashMap
    for utxo in user_utxos.iter() {
        let utxo_ref = utxo.as_ref();
        let txid = utxo_ref.txid_big_endian();

        // Linear search for existing txid (efficient for small collections)
        if let Some(existing_index) = txids.iter().position(|&existing| existing == txid) {
            user_txids_with_indices.push((txid, existing_index));
        } else {
            let new_index = txids.len();
            txids.push(txid);
            user_txids_with_indices.push((txid, new_index));
        }
    }

    // Track account txids and their indices
    let mut account_txids_with_indices = FixedList::<([u8; 32], usize), MAX_ACCOUNTS>::new();

    if let Some(accounts) = accounts {
        for account in accounts.iter() {
            let txid = account.utxo.txid_big_endian();

            // Linear search for existing txid
            if let Some(existing_index) = txids.iter().position(|&existing| existing == txid) {
                account_txids_with_indices.push((txid, existing_index));
            } else {
                let new_index = txids.len();
                txids.push(txid);
                account_txids_with_indices.push((txid, new_index));
            }
        }
    }

    // Get mempool data with optimized txids list
    let mempool_entries = exec_mempool_cpi(mempool_info_program_id, mempool_info_pdas, txids)
        .map_err(|e| ProgramError::Custom(e.into()))?;

    // Process the data with pre-computed indices (no HashMap needed)
    Ok(
        from_returned_data_to_mempool_data::<MAX_UTXOS, MAX_ACCOUNTS>(
            mempool_entries,
            user_txids_with_indices.as_slice(),
            if account_txids_with_indices.is_empty() {
                None
            } else {
                Some(account_txids_with_indices.as_slice())
            },
        ),
    )
}

fn from_returned_data_to_mempool_data<const MAX_UTXOS: usize, const MAX_ACCOUNTS: usize>(
    mempool: Vec<Option<ReturnedMempoolEntry>>,
    user_txids_with_indices: &[([u8; 32], usize)],
    account_txids_with_indices: Option<&[([u8; 32], usize)]>,
) -> MempoolData<MAX_UTXOS, MAX_ACCOUNTS> {
    // Prepare fixed-size buffers
    let mut utxo_mempool_info: [Option<([u8; 32], MempoolInfo)>; MAX_UTXOS] = [None; MAX_UTXOS];
    let mut utxo_index = 0usize;

    let mut accounts: [AccountMempoolInfo; MAX_ACCOUNTS] =
        std::array::from_fn(|_| AccountMempoolInfo::default());
    let mut accounts_i = 0;

    // Process user UTXOs using pre-computed indices
    for &(txid, index) in user_txids_with_indices.iter() {
        if utxo_index >= MAX_UTXOS {
            break; // Safety – should never happen (max user utxos)
        }

        if let Some(entry) = mempool[index].as_ref() {
            utxo_mempool_info[utxo_index] = Some((
                txid,
                MempoolInfo {
                    total_fee: entry.total_fee,
                    total_size: entry.total_vsize,
                },
            ));
            utxo_index += 1;
        }
    }

    // Process account accounts using pre-computed indices
    if let Some(account_txids_with_indices) = account_txids_with_indices {
        for &(_, index) in account_txids_with_indices.iter() {
            if accounts_i >= MAX_ACCOUNTS {
                break;
            }

            if let Some(entry) = mempool[index].as_ref() {
                accounts[accounts_i] = AccountMempoolInfo {
                    descendants_count: entry.descendants,
                    ancestors_count: entry.ancestors,
                };
            } else {
                accounts[accounts_i] = AccountMempoolInfo {
                    descendants_count: 0,
                    ancestors_count: 0,
                };
            }
            accounts_i += 1;
        }
    }

    MempoolData::new(utxo_mempool_info, accounts)
}

#[cfg(test)]
mod tests {
    use super::*;

    const MAX_UTXOS: usize = 3;
    const MAX_ACCOUNTS: usize = 2;

    fn sample_entry(
        total_fee: u64,
        total_vsize: u64,
        descendants: u16,
        ancestors: u16,
    ) -> ReturnedMempoolEntry {
        ReturnedMempoolEntry {
            total_fee,
            total_vsize,
            descendants,
            ancestors,
        }
    }

    #[test]
    fn test_from_returned_data_to_mempool_data() {
        // -----------------------------
        // Prepare test data
        // -----------------------------
        let txid_user_1 = [1u8; 32];
        let txid_user_2 = [2u8; 32];
        let txid_user_3 = [3u8; 32];

        let txid_account_1 = [10u8; 32];
        let txid_account_2 = [11u8; 32];

        // Index mapping mirrors the position inside the mempool vector
        let user_txids_with_indices = [
            (txid_user_1, 0usize),
            (txid_user_2, 1usize),
            (txid_user_3, 2usize),
        ];
        let account_txids_with_indices = [(txid_account_1, 3usize), (txid_account_2, 4usize)];

        // Build a mempool vector where some entries are present (Some) and some are missing (None)
        let mempool: Vec<Option<ReturnedMempoolEntry>> = vec![
            Some(sample_entry(100, 150, 1, 0)), // idx 0 => txid_user_1
            Some(sample_entry(200, 250, 2, 1)), // idx 1 => txid_user_2
            None,                               // idx 2 => txid_user_3  (confirmed)
            Some(sample_entry(300, 400, 5, 7)), // idx 3 => txid_account_1
            None,                               // idx 4 => txid_account_2 (no mempool info)
        ];

        // -----------------------------
        // Execute conversion helper
        // -----------------------------
        let mempool_data: MempoolData<MAX_UTXOS, MAX_ACCOUNTS> =
            super::from_returned_data_to_mempool_data::<MAX_UTXOS, MAX_ACCOUNTS>(
                mempool,
                &user_txids_with_indices,
                Some(&account_txids_with_indices),
            );

        // -----------------------------
        // Verify user-UTXO statuses
        // -----------------------------
        match mempool_data.get_utxo_status(txid_user_1) {
            TxStatus::Pending(info) => {
                assert_eq!(info.total_fee, 100);
                assert_eq!(info.total_size, 150);
            }
            _ => panic!("Expected pending status for txid_user_1"),
        }

        match mempool_data.get_utxo_status(txid_user_2) {
            TxStatus::Pending(info) => {
                assert_eq!(info.total_fee, 200);
                assert_eq!(info.total_size, 250);
            }
            _ => panic!("Expected pending status for txid_user_2"),
        }

        // txid_user_3 has no mempool info ⇒ should be confirmed
        assert!(matches!(
            mempool_data.get_utxo_status(txid_user_3),
            TxStatus::Confirmed
        ));

        // -----------------------------
        // Verify account-level mempool information
        // -----------------------------
        let accounts_slice = mempool_data.get_mempool_info_for_accounts(2);
        assert_eq!(accounts_slice.len(), 2);

        // account 0 should reflect the provided mempool entry (descendants & ancestors)
        assert_eq!(accounts_slice[0].descendants_count, 5);
        assert_eq!(accounts_slice[0].ancestors_count, 7);

        // account 1 had no mempool entry ⇒ counts should be zero
        assert_eq!(accounts_slice[1].descendants_count, 0);
        assert_eq!(accounts_slice[1].ancestors_count, 0);
    }

    #[test]
    fn test_get_utxo_status_confirmed_for_unknown_txid() {
        let mempool_data: MempoolData<MAX_UTXOS, MAX_ACCOUNTS> = MempoolData::default();
        let unknown_txid = [42u8; 32];
        assert!(matches!(
            mempool_data.get_utxo_status(unknown_txid),
            TxStatus::Confirmed
        ));
    }
}
