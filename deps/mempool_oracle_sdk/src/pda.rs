use arch_program::{account::AccountInfo, program_error::ProgramError, pubkey::Pubkey};

#[derive(Debug, thiserror::Error)]
pub enum PdaError {
    #[error("Invalid PDA: {0} - {1}")]
    InvalidPda(Pubkey, Pubkey),

    #[error("Invalid number of accounts: {0} - {1}")]
    InvalidAccounts(usize, u32),
}

/// Return the PDA (and bump) for a mempool-oracle state account `idx`.
///
/// Seed layout: `idx.to_le_bytes()`
#[inline]
pub fn mempool_pda_address(program_id: &Pubkey, idx: u32) -> (Pubkey, u8) {
    let idx_seed = idx.to_le_bytes();
    let seeds: [&[u8]; 1] = [&idx_seed];

    Pubkey::find_program_address(seeds.as_ref(), program_id)
}

/// Helper that returns all PDAs for a given program assuming the sequential
/// seed scheme described in [`mempool_pda_address`].
#[inline]
pub fn find_all_accounts(program_id: Pubkey, accounts: u32) -> Vec<(Pubkey, u8)> {
    (0..accounts)
        .map(|i| mempool_pda_address(&program_id, i))
        .collect()
}

/// Fixed-size variant that returns an array of `(Pubkey, bump)` pairs.
#[inline]
pub fn find_fixed_accounts<const N: usize>(program_id: Pubkey) -> [(Pubkey, u8); N] {
    let mut result = [(Pubkey::default(), 0); N];
    for i in 0..N {
        result[i] = mempool_pda_address(&program_id, i as u32);
    }
    result
}

/// Verify that each entry in `accounts` matches the PDA derived on-the-fly for
/// its index. Returns `Ok(())` on success or a [`ProgramError::Custom`] with
/// [`ErrorCode::InvalidPda`] (value `1`) on the first mismatch.
#[inline]
pub fn check_pda_accounts(
    program_id: Pubkey,
    accounts: &[AccountInfo<'static>],
    expected_accounts: u32,
) -> Result<(), ProgramError> {
    if accounts.len() != expected_accounts as usize {
        return Err(ProgramError::Custom(1)); // InvalidAccounts
    }

    for (idx, account) in accounts.iter().enumerate() {
        let (expected, _bump) = mempool_pda_address(&program_id, idx as u32);
        if *account.key != expected {
            return Err(ProgramError::Custom(1)); // InvalidPda
        }
    }

    Ok(())
}

#[macro_export]
macro_rules! define_idx_signer_seeds {
    ($var:ident, $idx:expr, $bump:expr) => {
        let idx_bytes = ($idx as u32).to_le_bytes();
        let seed_slice: &[&[u8]] = &[&idx_bytes];
        let bump_slice: &[&[u8]] = &[&[$bump]];
        let $var: &[&[&[u8]]] = &[seed_slice, bump_slice];
    };
}
