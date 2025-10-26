//! I/O adapters for CRLF normalization.
//!
//! This module provides `std::io::Read` and `std::io::Write` adapters that
//! normalizes incoming bytes to CRLF on-the-fly. Useful when you want to
//! stream-normalize data without materializing the whole input or output.
use std::io::{Read, Write};

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

    /// Consume the `NormalizingReader`, returning the underlying reader.
    /// 
    /// Note that there may be buffered bytes which are not re-acquired as part
    /// of this transition. It’s recommended to only call this function after EOF
    /// has been reached.
    pub fn into_inner(self) -> R {
        self.inner.into_inner()
    }
}

impl<R: Read> Read for NormalizingReader<R> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        self.inner.read(buf)
    }
}

pub struct NormalizingWriter<W> {
    inner: crate::io::NormalizingWriter<W>,
}

impl<W: Write> NormalizingWriter<W> {
    /// Create a new `NormalizingWriter` wrapping `inner`.
    ///
    /// Uses a sensible default internal buffer size.
    pub fn new(inner: W) -> Self {
        Self {
            inner: crate::io::NormalizingWriter::new(inner, crate::crlf::normalize_chunk),
        }
    }

    /// Create a `NormalizingWriter` with explicit internal buffer size.
    ///
    /// `buf_size` controls the size of the internal input buffer; the output
    /// buffer is sized accordingly. Larger buffers reduce syscalls and can
    /// increase throughput.
    pub fn with_size(inner: W, buf_size: usize) -> Self {
        Self {
            inner: crate::io::NormalizingWriter::with_size(
                inner,
                crate::crlf::normalize_chunk,
                buf_size,
            ),
        }
    }

    /// Finish writing and return the underlying writer.
    pub fn finish(self) -> std::io::Result<W> {
        self.inner.finish()
    }
}

impl<W: Write> Write for NormalizingWriter<W> {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.inner.write(buf)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.inner.flush()
    }
}
