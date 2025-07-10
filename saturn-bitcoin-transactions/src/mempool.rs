use arch_program::rune::RuneAmount;
use mempool_oracle_sdk::{MempoolData, MempoolInfo, TxStatus};
use saturn_collections::generic::fixed_set::FixedCapacitySet;

use crate::UtxoInfo;

pub(crate) fn generate_mempool_info<
    const MAX_UTXOS: usize,
    const MAX_ACCOUNTS: usize,
    RuneSet: FixedCapacitySet<Item = RuneAmount>,
>(
    user_utxos: &[UtxoInfo<RuneSet>],
    mempool_data: &MempoolData<MAX_UTXOS, MAX_ACCOUNTS>,
) -> MempoolInfo {
    let mut mempool_info = MempoolInfo::default();
    for (i, utxo) in user_utxos.iter().enumerate() {
        let txid: [u8; 32] = utxo.meta.txid_big_endian();

        // Check if we've already processed this txid in previous UTXOs
        let already_processed = user_utxos[..i].iter().any(|prev_utxo| {
            let prev_txid: [u8; 32] = prev_utxo.meta.txid_big_endian();
            prev_txid == txid
        });

        if already_processed {
            continue;
        }

        let status = mempool_data.get_utxo_status(txid);

        match status {
            TxStatus::Pending(info) => {
                mempool_info.total_fee += info.total_fee;
                mempool_info.total_size += info.total_size;
            }
            TxStatus::Confirmed => {}
        }
    }

    mempool_info
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::utxo_info::{SingleRuneSet, UtxoInfoTrait};
    use arch_program::utxo::UtxoMeta;
    use mempool_oracle_sdk::{AccountMempoolInfo, MempoolData, MempoolInfo};

    /// Convenience helper to construct a mock `UtxoInfo` with the desired txid/vout.
    fn make_utxo(txid: [u8; 32], vout: u32) -> UtxoInfo<SingleRuneSet> {
        UtxoInfo::new(UtxoMeta::from(txid, vout), 0)
    }

    #[test]
    fn aggregates_fees_and_sizes_for_unique_pending_txids() {
        const MAX_UTXOS: usize = 4;
        const MAX_ACCOUNTS: usize = 1;

        // Two distinct pending transactions in the mempool.
        let txid1 = [1u8; 32];
        let txid2 = [2u8; 32];

        let mut utxo_mempool_info: [Option<([u8; 32], MempoolInfo)>; MAX_UTXOS] = [None; MAX_UTXOS];
        utxo_mempool_info[0] = Some((
            txid1,
            MempoolInfo {
                total_fee: 100,
                total_size: 200,
            },
        ));
        utxo_mempool_info[1] = Some((
            txid2,
            MempoolInfo {
                total_fee: 50,
                total_size: 75,
            },
        ));

        let accounts_info: [AccountMempoolInfo; MAX_ACCOUNTS] =
            std::array::from_fn(|_| AccountMempoolInfo::default());

        let mempool_data =
            MempoolData::<MAX_UTXOS, MAX_ACCOUNTS>::new(utxo_mempool_info, accounts_info);

        let user_utxos = vec![make_utxo(txid1, 0), make_utxo(txid2, 1)];

        let info = generate_mempool_info::<MAX_UTXOS, MAX_ACCOUNTS, SingleRuneSet>(
            &user_utxos,
            &mempool_data,
        );

        assert_eq!(info.total_fee, 150);
        assert_eq!(info.total_size, 275);
    }

    #[test]
    fn ignores_duplicate_txids() {
        const MAX_UTXOS: usize = 4;
        const MAX_ACCOUNTS: usize = 1;

        let txid = [42u8; 32];

        let mut utxo_mempool_info: [Option<([u8; 32], MempoolInfo)>; MAX_UTXOS] = [None; MAX_UTXOS];
        utxo_mempool_info[0] = Some((
            txid,
            MempoolInfo {
                total_fee: 80,
                total_size: 160,
            },
        ));

        let accounts_info: [AccountMempoolInfo; MAX_ACCOUNTS] =
            std::array::from_fn(|_| AccountMempoolInfo::default());
        let mempool_data =
            MempoolData::<MAX_UTXOS, MAX_ACCOUNTS>::new(utxo_mempool_info, accounts_info);

        // Two UTXOs share the same transaction id but different vouts.
        let user_utxos = vec![make_utxo(txid, 0), make_utxo(txid, 1)];

        let info = generate_mempool_info::<MAX_UTXOS, MAX_ACCOUNTS, SingleRuneSet>(
            &user_utxos,
            &mempool_data,
        );

        assert_eq!(info.total_fee, 80);
        assert_eq!(info.total_size, 160);
    }

    #[test]
    fn ignores_confirmed_transactions() {
        const MAX_UTXOS: usize = 4;
        const MAX_ACCOUNTS: usize = 1;

        let pending_txid = [7u8; 32];
        let confirmed_txid = [8u8; 32];

        let mut utxo_mempool_info: [Option<([u8; 32], MempoolInfo)>; MAX_UTXOS] = [None; MAX_UTXOS];
        utxo_mempool_info[0] = Some((
            pending_txid,
            MempoolInfo {
                total_fee: 30,
                total_size: 60,
            },
        ));

        let accounts_info: [AccountMempoolInfo; MAX_ACCOUNTS] =
            std::array::from_fn(|_| AccountMempoolInfo::default());
        let mempool_data =
            MempoolData::<MAX_UTXOS, MAX_ACCOUNTS>::new(utxo_mempool_info, accounts_info);

        // Only one UTXO is pending, the other txid is not in the mempool (confirmed).
        let user_utxos = vec![make_utxo(pending_txid, 0), make_utxo(confirmed_txid, 0)];

        let info = generate_mempool_info::<MAX_UTXOS, MAX_ACCOUNTS, SingleRuneSet>(
            &user_utxos,
            &mempool_data,
        );

        assert_eq!(info.total_fee, 30);
        assert_eq!(info.total_size, 60);
    }
}
