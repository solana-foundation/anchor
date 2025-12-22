use crate::solana_program::program_memory::sol_memcpy;
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

    /// Returns the current write position (number of bytes written).
    pub fn position(&self) -> u64 {
        self.pos
    }
}

impl Write for BpfWriter<&mut [u8]> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bpf_writer_position() {
        let mut buffer = vec![0u8; 100];
        let mut writer = BpfWriter::new(buffer.as_mut_slice());

        // Initially position should be 0
        assert_eq!(writer.position(), 0);

        // Write some data
        let data = b"hello";
        writer.write_all(data).unwrap();
        assert_eq!(writer.position(), 5);

        // Write more data
        let more_data = b"world";
        writer.write_all(more_data).unwrap();
        assert_eq!(writer.position(), 10);

        // Verify the data was written correctly
        assert_eq!(&buffer[0..10], b"helloworld");
    }

    #[test]
    fn test_padding_scenario_without_fix() {
        // Simulate the issue

        let mut buffer = vec![0xFFu8; 20]; // Fill with 0xFF to simulate old data
        let original_len = buffer.len();

        let mut writer = BpfWriter::new(buffer.as_mut_slice());
        let new_data = vec![1u8; 15];
        writer.write_all(&new_data).unwrap();

        let written_len = writer.position() as usize;
        assert_eq!(written_len, 15);
        assert!(written_len < original_len);

        // No padding happens, so old data remains!
        assert_eq!(&buffer[0..15], &new_data); // New data written correctly
        assert_eq!(&buffer[15..20], &[0xFFu8; 5]); // OLD DATA!
    }

    #[test]
    fn test_padding_scenario_with_fix() {
        // Simulate the fix:

        let mut buffer = vec![0xFFu8; 20];
        let original_len = buffer.len();

        // Write only 15 bytes (simulating shrink)
        let mut writer = BpfWriter::new(buffer.as_mut_slice());
        let new_data = vec![1u8; 15];
        writer.write_all(&new_data).unwrap();

        let written_len = writer.position() as usize;
        assert_eq!(written_len, 15);
        assert!(written_len < original_len);
        let remaining = &mut buffer[written_len..];
        remaining.fill(0);

        // Verify padding worked
        assert_eq!(&buffer[0..15], &new_data); // Original data preserved
        assert_eq!(&buffer[15..20], &[0u8; 5]); // Remaining bytes zeroed
    }
}
