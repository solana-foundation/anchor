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

    // Stack-allocated seed array (max 16 seeds per Solana spec)
    assert!(seeds.len() <= 16, "PDA seeds exceed maximum of 16");
    let mut pino_seeds: [core::mem::MaybeUninit<pinocchio::cpi::Seed>; 16] =
        unsafe { core::mem::MaybeUninit::uninit().assume_init() };
    for (i, s) in seeds.iter().enumerate() {
        pino_seeds[i].write(pinocchio::cpi::Seed::from(*s));
    }
    // SAFETY: first `seeds.len()` elements are initialized above
    let initialized = unsafe {
        core::slice::from_raw_parts(pino_seeds.as_ptr() as *const pinocchio::cpi::Seed, seeds.len())
    };
    let signer = pinocchio::cpi::Signer::from(initialized);

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

/// Realloc an account to a new size, adjusting rent as needed.
pub fn realloc_account(
    account: &mut AccountView,
    new_space: usize,
    payer: &AccountView,
    zero: bool,
) -> Result<(), ProgramError> {
    use pinocchio::Resize;

    let old_space = account.data_len();
    let required = rent_exempt_lamports(new_space);
    let current_lamports = account.lamports();

    if new_space > old_space {
        let deficit = required.saturating_sub(current_lamports);
        if deficit > 0 {
            pinocchio_system::instructions::Transfer {
                from: payer, to: account, lamports: deficit,
            }.invoke()?;
        }
    } else if new_space < old_space {
        let excess = current_lamports.saturating_sub(required);
        if excess > 0 {
            let mut payer_mut = *payer;
            payer_mut.set_lamports(payer_mut.lamports() + excess);
            account.set_lamports(required);
        }
    }

    account.resize(new_space)?;

    if zero && new_space > old_space {
        unsafe {
            let data = account.borrow_unchecked_mut();
            for byte in &mut data[old_space..new_space] {
                *byte = 0;
            }
        }
    }

    Ok(())
}
