use {
    pinocchio::{
        account::AccountView,
        address::Address,
        sysvars::rent::{ACCOUNT_STORAGE_OVERHEAD, DEFAULT_LAMPORTS_PER_BYTE},
    },
    solana_program_error::ProgramError,
};

fn rent_exempt_lamports(space: usize) -> u64 {
    // TODO: investigate why `Rent::get()` returns zeros on surfpool/LiteSVM.
    (ACCOUNT_STORAGE_OVERHEAD + space as u64) * DEFAULT_LAMPORTS_PER_BYTE
}

/// Find a program-derived address (PDA).
pub fn find_program_address(seeds: &[&[u8]], program_id: &Address) -> (Address, u8) {
    Address::find_program_address(seeds, program_id)
}

/// Create a new account via system program CPI (no PDA signing).
pub fn create_account(
    payer: &AccountView,
    target: &AccountView,
    space: usize,
    owner: &Address,
) -> Result<(), ProgramError> {
    let required = rent_exempt_lamports(space);
    let current = target.lamports();

    if current == 0 {
        pinocchio_system::instructions::CreateAccount {
            from: payer, to: target, lamports: required, space: space as u64, owner,
        }.invoke()?;
    } else {
        create_prefunded(payer, target, space, owner, required, current, &[])?;
    }
    Ok(())
}

/// Create a new PDA account via system program CPI with signer seeds.
///
/// `seeds` should include the bump byte, e.g. `&[b"market", id.as_ref(), &[bump]]`.
pub fn create_account_signed(
    payer: &AccountView,
    target: &AccountView,
    space: usize,
    owner: &Address,
    seeds: &[&[u8]],
) -> Result<(), ProgramError> {
    let required = rent_exempt_lamports(space);
    let current = target.lamports();

    // Build pinocchio Seed array from raw seed slices
    let pino_seeds: Vec<pinocchio::cpi::Seed> = seeds.iter()
        .map(|s| pinocchio::cpi::Seed::from(*s))
        .collect();
    let signer = pinocchio::cpi::Signer::from(pino_seeds.as_slice());

    if current == 0 {
        pinocchio_system::instructions::CreateAccount {
            from: payer, to: target, lamports: required, space: space as u64, owner,
        }.invoke_signed(&[signer])?;
    } else {
        create_prefunded(payer, target, space, owner, required, current, &[signer])?;
    }
    Ok(())
}

fn create_prefunded(
    payer: &AccountView,
    target: &AccountView,
    space: usize,
    owner: &Address,
    required: u64,
    current: u64,
    signers: &[pinocchio::cpi::Signer],
) -> Result<(), ProgramError> {
    let top_up = required.saturating_sub(current);
    if top_up > 0 {
        pinocchio_system::instructions::Transfer {
            from: payer, to: target, lamports: top_up,
        }.invoke()?;
    }
    pinocchio_system::instructions::Allocate {
        account: target, space: space as u64,
    }.invoke_signed(signers)?;
    pinocchio_system::instructions::Assign {
        account: target, owner,
    }.invoke_signed(signers)?;
    Ok(())
}
