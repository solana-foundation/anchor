use {
    pinocchio::{
        account::AccountView,
        address::Address,
        sysvars::rent::{ACCOUNT_STORAGE_OVERHEAD, DEFAULT_LAMPORTS_PER_BYTE},
    },
    solana_program_error::ProgramError,
};

/// Largest `space` for which `rent_exempt_lamports` is guaranteed not to
/// overflow `u64`. Computed from pinocchio's rate constants — when those
/// change, this bound updates automatically.
///
/// `(MAX_SAFE_SPACE + ACCOUNT_STORAGE_OVERHEAD) * DEFAULT_LAMPORTS_PER_BYTE`
/// is the largest expression that fits in `u64`. Anything beyond this
/// would wrap; in practice the Solana runtime caps account data at 10 MiB
/// (~262× smaller than this bound) so the precondition is essentially
/// unreachable from honest callers.
const MAX_SAFE_SPACE: u64 =
    (u64::MAX / DEFAULT_LAMPORTS_PER_BYTE) - ACCOUNT_STORAGE_OVERHEAD;

/// Compute the rent-exempt minimum balance for an account of `space` bytes.
///
/// `const fn` so that callers passing a constant `space` (the common case —
/// `<MyAccount>::SPACE`, etc.) get the entire expression folded at compile
/// time. Runtime callers get a single forward-predicted compare against
/// `MAX_SAFE_SPACE` plus a 64-bit `mul`; the overflow-panic branch is
/// `#[cold]` so the optimizer pushes it out of the hot path.
///
/// # Panics
///
/// If `space > MAX_SAFE_SPACE`. The Solana runtime's 10 MiB account data
/// cap is well below this bound, so this is unreachable from honest
/// callers. The check is real (not a debug assert) so a misuse fails
/// loudly rather than silently producing wrapped lamports.
#[inline(always)]
pub const fn rent_exempt_lamports(space: usize) -> u64 {
    if space as u64 > MAX_SAFE_SPACE {
        rent_exempt_overflow();
    }
    // Bounded by MAX_SAFE_SPACE → no overflow → cheap 64-bit `mul`.
    (ACCOUNT_STORAGE_OVERHEAD + space as u64).wrapping_mul(DEFAULT_LAMPORTS_PER_BYTE)
}

#[cold]
#[inline(never)]
const fn rent_exempt_overflow() -> ! {
    panic!("rent_exempt_lamports: space exceeds u64 lamport capacity")
}

/// Find a program-derived address (PDA) and its bump seed.
///
/// Uses raw `sol_sha256` + `sol_curve_validate_point` syscalls directly instead
/// of the higher-level `sol_try_find_program_address` syscall, reducing per-attempt
/// cost from ~1,500 CU to ~544 CU.
///
/// Based on Quasar's implementation:
/// https://github.com/blueshift-gg/quasar/blob/8a62367/lang/src/pda.rs#L93-L182
#[inline(always)]
pub fn find_program_address(seeds: &[&[u8]], program_id: &Address) -> (Address, u8) {
    match try_find_program_address(seeds, program_id) {
        Ok(result) => result,
        Err(_) => panic!("could not find PDA"),
    }
}

/// Find a program-derived address, returning an error if none exists.
///
/// Uses raw `sol_sha256` + `sol_curve_validate_point` syscalls for ~3x lower CU
/// than `sol_try_find_program_address`.
///
/// Based on Quasar's implementation:
/// https://github.com/blueshift-gg/quasar/blob/8a62367/lang/src/pda.rs#L93-L182
#[inline(always)]
pub fn try_find_program_address(
    seeds: &[&[u8]],
    program_id: &Address,
) -> Result<(Address, u8), ProgramError> {
    if seeds.len() > 16 {
        return Err(ProgramError::InvalidSeeds);
    }

    #[cfg(target_os = "solana")]
    {
        use solana_define_syscall::definitions::{sol_curve_validate_point, sol_sha256};

        const CURVE25519_EDWARDS: u64 = 0;
        const PDA_MARKER: &[u8; 21] = b"ProgramDerivedAddress";

        let n = seeds.len();

        // Build the input array: [seeds..., bump, program_id, PDA_MARKER].
        // Max 16 seeds + bump + program_id + marker = 19 entries.
        let mut slices = core::mem::MaybeUninit::<[&[u8]; 19]>::uninit();
        let sptr = slices.as_mut_ptr() as *mut &[u8];

        let mut i = 0;
        while i < n {
            unsafe { sptr.add(i).write(seeds[i]) };
            i += 1;
        }
        unsafe {
            sptr.add(n + 1).write(program_id.as_ref());
            sptr.add(n + 2).write(PDA_MARKER.as_slice());
        }

        // The bump slot points into bump_arr — only the byte changes per iteration.
        let mut bump_arr = [u8::MAX];
        let bump_ptr = bump_arr.as_mut_ptr();
        unsafe { sptr.add(n).write(core::slice::from_raw_parts(bump_ptr, 1)) };

        let input = unsafe { core::slice::from_raw_parts(sptr, n + 3) };
        let mut hash = core::mem::MaybeUninit::<[u8; 32]>::uninit();

        let mut bump: u64 = u8::MAX as u64;

        loop {
            unsafe { bump_ptr.write(bump as u8) };

            unsafe {
                sol_sha256(
                    input as *const _ as *const u8,
                    input.len() as u64,
                    hash.as_mut_ptr() as *mut u8,
                );
            }

            // Returns 0 if on curve, non-zero if off curve (valid PDA).
            let on_curve = unsafe {
                sol_curve_validate_point(
                    CURVE25519_EDWARDS,
                    hash.as_ptr() as *const u8,
                    core::ptr::null_mut(),
                )
            };

            if on_curve != 0 {
                let hash_bytes = unsafe { hash.assume_init() };
                return Ok((Address::new_from_array(hash_bytes), bump as u8));
            }

            if bump == 0 {
                break;
            }
            bump -= 1;
        }

        Err(ProgramError::InvalidSeeds)
    }

    #[cfg(not(target_os = "solana"))]
    {
        // Off-chain fallback: use the standard SDK implementation.
        Ok(Address::find_program_address(seeds, program_id))
    }
}

/// Verify a program-derived address (PDA) using a known bump seed.
///
/// Uses `sol_sha256` directly (~200 CU) instead of `sol_create_program_address`
/// (~1,500 CU). The seeds slice should already include the bump byte.
///
/// Based on Quasar's implementation:
/// https://github.com/blueshift-gg/quasar/blob/8a62367/lang/src/pda.rs#L23-L84
#[inline(always)]
pub fn create_program_address(seeds: &[&[u8]], program_id: &Address) -> Result<Address, ProgramError> {
    #[cfg(target_os = "solana")]
    {
        Ok(hash_pda_seeds(seeds, program_id)?)
    }

    #[cfg(not(target_os = "solana"))]
    {
        Address::create_program_address(seeds, program_id).map_err(Into::into)
    }
}

/// Verify that `expected` matches the PDA derived from `seeds` and `program_id`.
///
/// Skips the `sol_curve_validate_point` check — the bump is assumed valid
/// (it was derived during account creation). Only computes `sha256` and
/// compares the hash (~200 CU vs ~544 CU for find, ~350 CU for create).
///
/// Based on Quasar's `verify_program_address`:
/// https://github.com/blueshift-gg/quasar/blob/8a62367/lang/src/pda.rs#L23-L84
#[inline(always)]
pub fn verify_program_address(
    seeds: &[&[u8]],
    program_id: &Address,
    expected: &Address,
) -> Result<(), ProgramError> {
    #[cfg(target_os = "solana")]
    {
        let computed = hash_pda_seeds(seeds, program_id)?;
        if computed == *expected {
            Ok(())
        } else {
            Err(ProgramError::InvalidSeeds)
        }
    }

    #[cfg(not(target_os = "solana"))]
    {
        let computed = Address::create_program_address(seeds, program_id)
            .map_err(|_| ProgramError::InvalidSeeds)?;
        if computed == *expected {
            Ok(())
        } else {
            Err(ProgramError::InvalidSeeds)
        }
    }
}

/// Hash seeds into a PDA address (sha256 only, no curve check).
#[cfg(target_os = "solana")]
#[inline(always)]
fn hash_pda_seeds(seeds: &[&[u8]], program_id: &Address) -> Result<Address, ProgramError> {
    use solana_define_syscall::definitions::sol_sha256;
    const PDA_MARKER: &[u8; 21] = b"ProgramDerivedAddress";

    if seeds.len() > 17 {
        return Err(ProgramError::InvalidSeeds);
    }

    let n = seeds.len();
    let mut slices = core::mem::MaybeUninit::<[&[u8]; 19]>::uninit();
    let sptr = slices.as_mut_ptr() as *mut &[u8];

    let mut i = 0;
    while i < n {
        unsafe { sptr.add(i).write(seeds[i]) };
        i += 1;
    }
    unsafe {
        sptr.add(n).write(program_id.as_ref());
        sptr.add(n + 1).write(PDA_MARKER.as_slice());
    }

    let input = unsafe { core::slice::from_raw_parts(sptr, n + 2) };
    let mut hash = core::mem::MaybeUninit::<[u8; 32]>::uninit();

    unsafe {
        sol_sha256(
            input as *const _ as *const u8,
            input.len() as u64,
            hash.as_mut_ptr() as *mut u8,
        );
    }

    Ok(Address::new_from_array(unsafe { hash.assume_init() }))
}

/// Create a new account via system program CPI (no PDA signing).
#[inline(always)]
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
#[inline(always)]
pub fn create_account_signed(
    payer: &AccountView,
    target: &AccountView,
    space: usize,
    owner: &Address,
    seeds: &[&[u8]],
) -> Result<(), ProgramError> {
    let required = rent_exempt_lamports(space);
    let current = target.lamports();

    // SAFETY: `Seed` is repr(C) with layout (*const u8, u64, PhantomData) = 16 bytes,
    // identical to `&[u8]` on SBF (*const u8, u64) = 16 bytes. PhantomData is zero-sized.
    // This cast avoids copying seed data into a MaybeUninit array.
    let signer_seeds: &[pinocchio::cpi::Seed] = unsafe {
        core::slice::from_raw_parts(seeds.as_ptr() as *const pinocchio::cpi::Seed, seeds.len())
    };
    let signer = pinocchio::cpi::Signer::from(signer_seeds);

    if current == 0 {
        pinocchio_system::instructions::CreateAccount {
            from: payer, to: target, lamports: required, space: space as u64, owner,
        }.invoke_signed(&[signer])?;
    } else {
        create_prefunded(payer, target, space, owner, required, current, &[signer])?;
    }
    Ok(())
}

/// Rare-path fallback for when the target account already holds lamports
/// at creation time (e.g. airdropped PDAs or `init_if_needed` after partial
/// funding). Marked `#[cold]` + `#[inline(never)]` so LTO keeps it out of
/// the hot dispatch path and the ~1.4 KB of Transfer/Allocate/Assign CPI
/// glue doesn't bloat the `entrypoint` or per-instruction wrappers.
#[cold]
#[inline(never)]
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
            // `checked_add` rather than `+`: overflow-checks is disabled in
            // release builds, and this arithmetic is on user-supplied account
            // lamports. The total SOL supply is bounded so overflow is
            // unreachable in practice, but silent wrap would be a downgrade.
            let new_payer_lamports = payer_mut
                .lamports()
                .checked_add(excess)
                .ok_or(ProgramError::ArithmeticOverflow)?;
            payer_mut.set_lamports(new_payer_lamports);
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
