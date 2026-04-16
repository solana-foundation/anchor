#[cfg(feature = "const-rent")]
use pinocchio::sysvars::rent::{ACCOUNT_STORAGE_OVERHEAD, DEFAULT_LAMPORTS_PER_BYTE};
use {
    pinocchio::{account::AccountView, address::Address},
    solana_program_error::ProgramError,
};

/// Re-export pinocchio CPI building blocks for instruction construction.
pub use pinocchio::cpi::{Seed as CpiSeed, Signer as CpiSigner};
pub use pinocchio::instruction::{InstructionAccount, InstructionView};

/// Largest `space` for which the const path is guaranteed not to
/// overflow `u64`. Only referenced when the `const-rent` feature is on.
///
/// `(MAX_SAFE_SPACE + ACCOUNT_STORAGE_OVERHEAD) * DEFAULT_LAMPORTS_PER_BYTE`
/// is the largest expression that fits in `u64`. In practice the Solana
/// runtime caps account data at 10 MiB (~262× smaller than this bound) so
/// the precondition is essentially unreachable from honest callers.
#[cfg(feature = "const-rent")]
const MAX_SAFE_SPACE: u64 = (u64::MAX / DEFAULT_LAMPORTS_PER_BYTE) - ACCOUNT_STORAGE_OVERHEAD;

/// Compute the rent-exempt minimum balance for an account of `space` bytes.
///
/// Default path calls `Rent::get()` (picks up runtime formula changes).
/// With `const-rent` feature, uses baked-in constants (zero syscall cost
/// but locks the formula into the binary).
#[cfg(not(feature = "const-rent"))]
#[inline]
pub fn rent_exempt_lamports(space: usize) -> Result<u64, ProgramError> {
    use pinocchio::sysvars::{rent::Rent, Sysvar};
    Rent::get()?.try_minimum_balance(space)
}

#[cfg(feature = "const-rent")]
#[inline(always)]
pub fn rent_exempt_lamports(space: usize) -> Result<u64, ProgramError> {
    if space as u64 > MAX_SAFE_SPACE {
        return Err(ProgramError::InvalidArgument);
    }
    // Bounded by MAX_SAFE_SPACE → no overflow.
    Ok((ACCOUNT_STORAGE_OVERHEAD + space as u64).wrapping_mul(DEFAULT_LAMPORTS_PER_BYTE))
}

/// PDA bump-search loop. `$on_found` receives the hash bytes and bump
/// when a valid off-curve PDA is found.
#[cfg(target_os = "solana")]
macro_rules! pda_find_loop {
    ($seeds:expr, $program_id:expr, |$h:ident, $b:ident| $on_found:expr) => {{
        use solana_define_syscall::definitions::{sol_curve_validate_point, sol_sha256};
        const CURVE25519_EDWARDS: u64 = 0;
        const PDA_MARKER: &[u8; 21] = b"ProgramDerivedAddress";

        let n = $seeds.len();
        let mut slices = core::mem::MaybeUninit::<[&[u8]; 19]>::uninit();
        let sptr = slices.as_mut_ptr() as *mut &[u8];
        let mut i = 0;
        while i < n {
            unsafe { sptr.add(i).write($seeds[i]) };
            i += 1;
        }
        unsafe {
            sptr.add(n + 1).write($program_id.as_ref());
            sptr.add(n + 2).write(PDA_MARKER.as_slice());
        }
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
                )
            };
            let on_curve = unsafe {
                sol_curve_validate_point(
                    CURVE25519_EDWARDS,
                    hash.as_ptr() as *const u8,
                    core::ptr::null_mut(),
                )
            };
            if on_curve != 0 {
                let $h = unsafe { hash.assume_init() };
                let $b = bump as u8;
                return $on_found;
            }
            if bump == 0 {
                break;
            }
            bump -= 1;
        }
        Err(ProgramError::InvalidSeeds)
    }};
}

/// Find a PDA and its bump seed.
#[inline(always)]
pub fn find_program_address(seeds: &[&[u8]], program_id: &Address) -> (Address, u8) {
    match try_find_program_address(seeds, program_id) {
        Ok(result) => result,
        Err(_) => panic!("could not find PDA"),
    }
}

/// Find a PDA, returning an error if none exists.
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
        pda_find_loop!(seeds, program_id, |hash_bytes, bump| {
            Ok((Address::new_from_array(hash_bytes), bump))
        })
    }

    #[cfg(not(target_os = "solana"))]
    {
        Ok(Address::find_program_address(seeds, program_id))
    }
}

/// Find the canonical bump for `seeds` + `program_id` and verify that the
/// derived PDA equals `expected`. Returns just the bump on success.
#[inline(always)]
pub fn find_and_verify_program_address(
    seeds: &[&[u8]],
    program_id: &Address,
    expected: &Address,
) -> Result<u8, ProgramError> {
    if seeds.len() > 16 {
        return Err(ProgramError::InvalidSeeds);
    }

    #[cfg(target_os = "solana")]
    {
        pda_find_loop!(seeds, program_id, |hash_bytes, bump| {
            let derived = Address::new_from_array(hash_bytes);
            if pinocchio::address::address_eq(&derived, expected) {
                Ok(bump)
            } else {
                Err(ProgramError::InvalidSeeds)
            }
        })
    }

    #[cfg(not(target_os = "solana"))]
    {
        let (pda, bump) = Address::find_program_address(seeds, program_id);
        if pinocchio::address::address_eq(&pda, expected) {
            Ok(bump)
        } else {
            Err(ProgramError::InvalidSeeds)
        }
    }
}

/// Create a program-derived address (PDA) from `seeds` and `program_id`.
///
/// Uses `sol_sha256` + `sol_curve_validate_point` directly instead of
/// `sol_create_program_address`. The seeds slice should already include
/// the bump byte.
#[inline(always)]
pub fn create_program_address(
    seeds: &[&[u8]],
    program_id: &Address,
) -> Result<Address, ProgramError> {
    #[cfg(target_os = "solana")]
    {
        let computed = hash_pda_seeds(seeds, program_id)?;
        check_off_curve(&computed)?;
        Ok(computed)
    }

    #[cfg(not(target_os = "solana"))]
    {
        Address::create_program_address(seeds, program_id).map_err(Into::into)
    }
}

/// Verify that `expected` matches the PDA derived from `seeds` and `program_id`.
///
/// Computes `sha256` and compares the hash to `expected`. Does NOT
/// check if the point is off-curve — the bump is assumed to be canonical
/// (e.g. read from account data that was set during init, or precomputed at
/// compile time). Callers passing untrusted bump values should use
/// `find_and_verify_program_address` instead, which includes the curve check.
///
/// The seeds slice should already include the bump byte.
#[inline(always)]
pub fn verify_program_address(
    seeds: &[&[u8]],
    program_id: &Address,
    expected: &Address,
) -> Result<(), ProgramError> {
    #[cfg(target_os = "solana")]
    {
        let computed = hash_pda_seeds(seeds, program_id)?;
        if pinocchio::address::address_eq(&computed, expected) {
            Ok(())
        } else {
            Err(ProgramError::InvalidSeeds)
        }
    }

    #[cfg(not(target_os = "solana"))]
    {
        let computed = Address::create_program_address(seeds, program_id)
            .map_err(|_| ProgramError::InvalidSeeds)?;
        if pinocchio::address::address_eq(&computed, expected) {
            Ok(())
        } else {
            Err(ProgramError::InvalidSeeds)
        }
    }
}

/// Like [`find_and_verify_program_address`] but skips `sol_curve_validate_point`.
///
/// Safe when the caller can prove the account was signed for:
/// - **`MIN_DATA_LEN > 0`**: account has data → was created via
///   CreateAccount/Allocate (which require the account to be a signer).
///   For PDAs, signing goes through `invoke_signed` →
///   `create_program_address` which includes the runtime's curve check.
/// - **`init`**: account is system-owned with `data_len == 0` → the
///   subsequent CreateAccount/Allocate/Assign CPI requires signing →
///   same reasoning.
///
/// Without the curve check the loop tries all 256 bumps via hash-and-compare.
/// SHA-256 collision resistance ensures only the canonical bump matches
/// the expected on-chain address.
#[inline(always)]
pub fn find_and_verify_program_address_skip_curve(
    seeds: &[&[u8]],
    program_id: &Address,
    expected: &Address,
) -> Result<u8, ProgramError> {
    if seeds.len() > 16 {
        return Err(ProgramError::InvalidSeeds);
    }

    #[cfg(target_os = "solana")]
    {
        use solana_define_syscall::definitions::sol_sha256;
        const PDA_MARKER: &[u8; 21] = b"ProgramDerivedAddress";

        let n = seeds.len();
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
                )
            };
            let h = unsafe { hash.assume_init() };
            let derived = Address::new_from_array(h);
            if pinocchio::address::address_eq(&derived, expected) {
                return Ok(bump as u8);
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
        let (pda, bump) = Address::find_program_address(seeds, program_id);
        if pinocchio::address::address_eq(&pda, expected) {
            Ok(bump)
        } else {
            Err(ProgramError::InvalidSeeds)
        }
    }
}

/// Verify that `addr` is off the Ed25519 curve (i.e. a valid PDA).
/// Returns `InvalidSeeds` if the point is on-curve.
#[cfg(target_os = "solana")]
#[inline(always)]
fn check_off_curve(addr: &Address) -> Result<(), ProgramError> {
    use solana_define_syscall::definitions::sol_curve_validate_point;
    const CURVE25519_EDWARDS: u64 = 0;
    let on_curve = unsafe {
        sol_curve_validate_point(
            CURVE25519_EDWARDS,
            addr as *const _ as *const u8,
            core::ptr::null_mut(),
        )
    };
    if on_curve == 0 {
        Err(ProgramError::InvalidSeeds)
    } else {
        Ok(())
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
    let required = rent_exempt_lamports(space)?;
    let current = target.lamports();

    if current == 0 {
        pinocchio_system::instructions::CreateAccount {
            from: payer,
            to: target,
            lamports: required,
            space: space as u64,
            owner,
        }
        .invoke()?;
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
    let required = rent_exempt_lamports(space)?;
    let current = target.lamports();

    // SAFETY: Seed is repr(C) { *const u8, u64, PhantomData } = 16 bytes,
    // identical to &[u8] on SBF (*const u8, u64) = 16 bytes.
    // PhantomData is zero-sized. Static assertions verify at compile time.
    const _: () =
        assert!(core::mem::size_of::<&[u8]>() == core::mem::size_of::<pinocchio::cpi::Seed>());
    const _: () =
        assert!(core::mem::align_of::<&[u8]>() == core::mem::align_of::<pinocchio::cpi::Seed>());
    let signer_seeds: &[pinocchio::cpi::Seed] = unsafe {
        core::slice::from_raw_parts(seeds.as_ptr() as *const pinocchio::cpi::Seed, seeds.len())
    };
    let signer = pinocchio::cpi::Signer::from(signer_seeds);

    if current == 0 {
        pinocchio_system::instructions::CreateAccount {
            from: payer,
            to: target,
            lamports: required,
            space: space as u64,
            owner,
        }
        .invoke_signed(&[signer])?;
    } else {
        create_prefunded(payer, target, space, owner, required, current, &[signer])?;
    }
    Ok(())
}

/// Rare-path fallback for when the target account already holds lamports
/// at creation time (e.g. airdropped PDAs or `init_if_needed`).
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
    .invoke_signed(signers)?;
    pinocchio_system::instructions::Assign {
        account: target,
        owner,
    }
    .invoke_signed(signers)?;
    Ok(())
}

/// Realloc an account to a new size, adjusting rent as needed.
///
/// Requires the `account-resize` feature (default-on). Disabling that
/// feature also drops the pinocchio `account-resize` entrypoint hook that
/// writes `data_len` into `RuntimeAccount.padding` for every non-duplicate
/// account, so calling this without the hook would corrupt the stored
/// `original_data_len` that `AccountView::resize()` reads back. The two
/// have to move together, hence the compile-time gate rather than a
/// runtime error.
#[cfg(feature = "account-resize")]
pub fn realloc_account(
    account: &mut AccountView,
    new_space: usize,
    payer: &AccountView,
    zero: bool,
) -> Result<(), ProgramError> {
    use pinocchio::Resize;

    let old_space = account.data_len();
    let required = rent_exempt_lamports(new_space)?;
    let current_lamports = account.lamports();

    if new_space > old_space {
        let deficit = required.saturating_sub(current_lamports);
        if deficit > 0 {
            pinocchio_system::instructions::Transfer {
                from: payer,
                to: account,
                lamports: deficit,
            }
            .invoke()?;
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

    // SAFETY: realloc is called from derive-generated code on an AccountView
    // whose backing Slab holds the conceptual borrow (borrow_state == 0).
    // Using resize_unchecked bypasses pinocchio's check_borrow_mut() which
    // would reject the operation due to the Slab's borrow flag.
    unsafe { account.resize_unchecked(new_space)? };

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
