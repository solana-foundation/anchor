use crate::solana_program::program_memory::sol_memcpy;
use crate::{Write, WriteError};
use core::cmp;

#[derive(Debug, Default)]
pub struct BpfWriter<T> {
    inner: T,
    pos: u64,
}

impl<T> BpfWriter<T> {
    pub fn new(inner: T) -> Self {
        Self { inner, pos: 0 }
    }
}

impl Write for BpfWriter<&mut [u8]> {
    fn write(&mut self, buf: &[u8]) -> core::result::Result<usize, WriteError> {
        let remaining_inner = match self.inner.get_mut(self.pos as usize..) {
            Some(buf) if !buf.is_empty() => buf,
            _ => return Ok(0),
        };

        let amt = cmp::min(remaining_inner.len(), buf.len());
        // SAFETY: `amt` is guarenteed by the above line to be in bounds for both slices
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
