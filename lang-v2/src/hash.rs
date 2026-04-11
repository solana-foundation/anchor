/// SHA-256 hash of `data`.
///
/// On-chain (`target_os = "solana"`): `sol_sha256` syscall (~85 CU + zero
/// bytes of program text). Off-chain: `sha2` crate.
///
/// Going through this helper instead of pulling in `sha2` directly saves
/// ~54 KB of `.text` per program — `sha2::sha256::compress256` is the
/// dominant cost when it gets linked in.
#[inline]
pub fn sha256(data: &[u8]) -> [u8; 32] {
    #[cfg(target_os = "solana")]
    {
        use solana_define_syscall::definitions::sol_sha256;
        let slices: [&[u8]; 1] = [data];
        let mut out = core::mem::MaybeUninit::<[u8; 32]>::uninit();
        unsafe {
            sol_sha256(
                slices.as_ptr() as *const u8,
                1,
                out.as_mut_ptr() as *mut u8,
            );
            out.assume_init()
        }
    }
    #[cfg(not(target_os = "solana"))]
    {
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(data);
        hasher.finalize().into()
    }
}
