use solana_program::program_memory::sol_memcpy;
use std::cmp;
use std::io::{self, Write};

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
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        if self.pos >= self.inner.len() as u64 {
            return Ok(0);
        }

        let amt = cmp::min(
            self.inner.len().saturating_sub(self.pos as usize),
            buf.len(),
        );
        sol_memcpy(&mut self.inner[(self.pos as usize)..], buf, amt);
        self.pos += amt as u64;
        // Set remaining bytes to 0
        if pos < self.inner.len() {
            let remaining_bytes = self.inner.len() - pos;
            sol_memcpy(
                &mut self.inner[pos..],
                vec![0; remaining_bytes].as_ref(),
                remaining_bytes,
            );
            self.pos += remaining_bytes as u64;
        }

        Ok(amt)
    }

    fn write_all(&mut self, buf: &[u8]) -> io::Result<()> {
        if self.write(buf)? == buf.len() {
            Ok(())
        } else {
            Err(io::Error::new(
                io::ErrorKind::WriteZero,
                "failed to write whole buffer",
            ))
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
