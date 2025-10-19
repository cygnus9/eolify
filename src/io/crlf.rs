use std::io::Read;

/// I/O adapters for CRLF normalization.
///
/// This module provides a `std::io::Read` adapter that normalizes incoming
/// bytes to CRLF on-the-fly. Useful when you want to stream-normalize data
/// without materializing the whole input.
pub struct NormalizingReader<R> {
    inner: crate::io::NormalizingReader<R>,
}

impl<R: Read> NormalizingReader<R> {
    /// Create a new `NormalizingReader` wrapping `inner`.
    ///
    /// Uses a sensible default internal buffer size.
    pub fn new(inner: R) -> Self {
        Self {
            inner: crate::io::NormalizingReader::new(inner, crate::crlf::normalize_chunk),
        }
    }

    /// Create a `NormalizingReader` with explicit internal buffer size.
    ///
    /// `buf_size` controls the size of the internal input buffer; the output
    /// buffer is sized accordingly. Larger buffers reduce syscalls and can
    /// increase throughput.
    pub fn with_size(inner: R, buf_size: usize) -> Self {
        Self {
            inner: crate::io::NormalizingReader::with_size(
                inner,
                crate::crlf::normalize_chunk,
                buf_size,
            ),
        }
    }
}

impl<R: Read> Read for NormalizingReader<R> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        self.inner.read(buf)
    }
}
