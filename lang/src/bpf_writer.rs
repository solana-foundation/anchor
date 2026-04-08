#[cfg(not(feature = "std"))]
use crate::{Write, WriteError};
use {crate::solana_program::program_memory::sol_memcpy, core::cmp};

#[derive(Debug, Default)]
pub struct BpfWriter<T> {
    inner: T,
    pos: u64,
}

impl<T> BpfWriter<T> {
    pub fn new(inner: T) -> Self {
        Self { inner, pos: 0 }
    }

    /// Current write cursor into `inner` (bytes written so far).
    #[inline]
    pub fn position(&self) -> u64 {
        self.pos
    }
}

// With `feature = "std"`, `impl<T: std::io::Write + ?Sized> Write for T`.
// A direct `impl Write for BpfWriter<…>` would overlap that blanket (same type, two `Write` impls),
// so this type must expose I/O only through `std::io::Write`; [`crate::Write`] is then the blanket.
// This impl must contain the real memcpy logic (not call `Write::write`) or it would recurse into
// the blanket. On `no_std` there is no such blanket, so we implement [`crate::Write`] only, below.
#[cfg(feature = "std")]
impl std::io::Write for BpfWriter<&mut [u8]> {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let remaining_inner = match self.inner.get_mut(self.pos as usize..) {
            Some(b) if !b.is_empty() => b,
            _ => return Ok(0),
        };

        let amt = cmp::min(remaining_inner.len(), buf.len());
        // SAFETY: `amt` is guaranteed by the above line to be in bounds for both slices
        unsafe {
            sol_memcpy(remaining_inner, buf, amt);
        }
        self.pos += amt as u64;
        Ok(amt)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

// No `std::io::Write` bridge exists without `std`; bounded account serialization uses this impl only.
#[cfg(not(feature = "std"))]
impl Write for BpfWriter<&mut [u8]> {
    fn write(&mut self, buf: &[u8]) -> core::result::Result<usize, WriteError> {
        let remaining_inner = match self.inner.get_mut(self.pos as usize..) {
            Some(buf) if !buf.is_empty() => buf,
            _ => return Ok(0),
        };

        let amt = cmp::min(remaining_inner.len(), buf.len());
        // SAFETY: `amt` is guaranteed by the above line to be in bounds for both slices
        unsafe {
            sol_memcpy(remaining_inner, buf, amt);
        }
        self.pos += amt as u64;
        Ok(amt)
    }

    fn write_all(&mut self, buf: &[u8]) -> core::result::Result<(), WriteError> {
        if self.write(buf)? == buf.len() {
            Ok(())
        } else {
            Err(WriteError::WriteZero)
        }
    }

    fn flush(&mut self) -> core::result::Result<(), WriteError> {
        Ok(())
    }
}
