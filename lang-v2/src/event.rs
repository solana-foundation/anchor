use alloc::vec::Vec;
use crate::Discriminator;

/// Trait for event structs. Implemented by the `#[event]` attribute macro.
pub trait Event: borsh::BorshSerialize + Discriminator {
    /// Serialize the event: discriminator bytes followed by borsh-serialized data.
    fn data(&self) -> Vec<u8> {
        let mut data = Vec::with_capacity(256);
        data.extend_from_slice(Self::DISCRIMINATOR);
        borsh::BorshSerialize::serialize(self, &mut data).unwrap();
        data
    }
}

/// Log event data via the `sol_log_data` syscall.
///
/// On-chain (`target_os = "solana"`), this calls the `sol_log_data` syscall
/// which emits a `Program data: <base64>` log entry that clients can subscribe to.
///
/// Off-chain (tests / non-Solana), this is a no-op.
pub fn sol_log_data(data: &[&[u8]]) {
    #[cfg(target_os = "solana")]
    unsafe {
        pinocchio::syscalls::sol_log_data(
            data as *const _ as *const u8,
            data.len() as u64,
        )
    };

    #[cfg(not(target_os = "solana"))]
    core::hint::black_box(data);
}
