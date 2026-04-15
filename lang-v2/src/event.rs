use {crate::Discriminator, alloc::vec::Vec};

/// Trait for event structs. Implemented by the `#[event]` attribute macro.
pub trait Event: Discriminator {
    /// Serialized size of the event data (excluding discriminator).
    const DATA_SIZE: usize;

    /// Write the event data into the buffer (wincode zero-copy for fixed-size structs).
    fn write_data(&self, buf: &mut [u8]);

    /// Serialize the event: discriminator bytes followed by event data.
    fn data(&self) -> Vec<u8> {
        let disc = Self::DISCRIMINATOR;
        let mut data = Vec::with_capacity(disc.len() + Self::DATA_SIZE);
        data.extend_from_slice(disc);
        let start = data.len();
        data.resize(start + Self::DATA_SIZE, 0);
        self.write_data(&mut data[start..]);
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
    // SAFETY: data is a valid slice-of-slices; the syscall reads but does not write.
    unsafe {
        pinocchio::syscalls::sol_log_data(data as *const _ as *const u8, data.len() as u64)
    };

    #[cfg(not(target_os = "solana"))]
    core::hint::black_box(data);
}
