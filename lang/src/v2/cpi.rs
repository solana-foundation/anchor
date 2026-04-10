use {
    pinocchio::{
        account::AccountView,
        address::Address,
        sysvars::rent::{ACCOUNT_STORAGE_OVERHEAD, DEFAULT_LAMPORTS_PER_BYTE},
    },
    solana_program_error::ProgramError,
};

/// Create a new account via system program CPI.
///
/// Handles the case where the account already has lamports (e.g. from a
/// prior transfer). If lamports == 0, uses `CreateAccount`. Otherwise,
/// tops up lamports + `Allocate` + `Assign` (since `CreateAccount` fails
/// on accounts with existing lamports).
///
/// TODO: investigate why `Rent::get()` returns zeros on surfpool/LiteSVM.
/// Uses `DEFAULT_LAMPORTS_PER_BYTE` constant as a workaround.
pub fn create_account(
    payer: &AccountView,
    target: &AccountView,
    space: usize,
    owner: &Address,
) -> Result<(), ProgramError> {
    let required_lamports = (ACCOUNT_STORAGE_OVERHEAD + space as u64) * DEFAULT_LAMPORTS_PER_BYTE;
    let current_lamports = target.lamports();

    if current_lamports == 0 {
        pinocchio_system::instructions::CreateAccount {
            from: payer,
            to: target,
            lamports: required_lamports,
            space: space as u64,
            owner,
        }
        .invoke()?;
    } else {
        // Account already has lamports — can't use CreateAccount.
        // Top up, allocate, assign instead.
        let top_up = required_lamports.saturating_sub(current_lamports);
        if top_up > 0 {
            pinocchio_system::instructions::Transfer {
                from: payer,
                to: target,
                lamports: top_up,
            }
            .invoke()?;
        }
        pinocchio_system::instructions::Allocate {
            account: target,
            space: space as u64,
        }
        .invoke()?;
        pinocchio_system::instructions::Assign {
            account: target,
            owner,
        }
        .invoke()?;
    }

    Ok(())
}
