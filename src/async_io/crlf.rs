//! Async I/O adapters for CRLF normalization.
//!
//! This module provides `futures_io::AsyncRead` and `futures_io::AsyncWrite`
//! adapters that normalizes incoming bytes to CRLF on-the-fly. Useful when you
//! want to stream-normalize data without materializing the whole input or output.
use std::{
    pin::Pin,
    task::{Context, Poll},
};

// use async_std::future;
use pin_project_lite::pin_project;

// use crate::async_io::{FuturesIoReader, PollRead, TokioIoReader};

pin_project! {
    pub struct NormalizingReader<R> {
        #[pin]
        inner: crate::async_io::NormalizingReader<R>,
    }
}

#[cfg(feature = "futures-io")]
impl<R: futures_io::AsyncRead> NormalizingReader<R> {
    /// Create a new `NormalizingReader` wrapping `inner`.
    ///
    /// Uses a sensible default internal buffer size.
    pub fn futures_new(inner: R) -> Self {
        Self {
            inner: crate::async_io::NormalizingReader::new(inner, crate::crlf::normalize_chunk),
        }
    }

    /// Create a `NormalizingReader` with explicit internal buffer size.
    ///
    /// `buf_size` controls the size of the internal input buffer; the output
    /// buffer is sized accordingly. Larger buffers reduce syscalls and can
    /// increase throughput.
    pub fn futures_with_size(inner: R, buf_size: usize) -> Self {
        Self {
            inner: crate::async_io::NormalizingReader::with_size(
                inner,
                crate::crlf::normalize_chunk,
                buf_size,
            ),
        }
    }
}

#[cfg(feature = "tokio-io")]
impl<R: tokio::io::AsyncRead> NormalizingReader<R> {
    /// Create a new `NormalizingReader` wrapping `inner`.
    ///
    /// Uses a sensible default internal buffer size.
    pub fn tokio_new(inner: R) -> Self {
        Self {
            inner: crate::async_io::NormalizingReader::new(inner, crate::crlf::normalize_chunk),
        }
    }

    /// Create a `NormalizingReader` with explicit internal buffer size.
    ///
    /// `buf_size` controls the size of the internal input buffer; the output
    /// buffer is sized accordingly. Larger buffers reduce syscalls and can
    /// increase throughput.
    pub fn tokio_with_size(inner: R, buf_size: usize) -> Self {
        Self {
            inner: crate::async_io::NormalizingReader::with_size(
                inner,
                crate::crlf::normalize_chunk,
                buf_size,
            ),
        }
    }
}

impl<R> NormalizingReader<R> {
    /// Consume the `NormalizingReader`, returning the underlying reader.
    ///
    /// Note that there may be buffered bytes which are not re-acquired as part
    /// of this transition. It’s recommended to only call this function after EOF
    /// has been reached.
    pub fn into_inner(self) -> R {
        self.inner.into_inner()
    }
}

#[cfg(feature = "futures-io")]
impl<R: futures_io::AsyncRead> futures_io::AsyncRead for NormalizingReader<R> {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut [u8],
    ) -> Poll<std::io::Result<usize>> {
        self.project().inner.poll_read(cx, buf)
    }
}

#[cfg(feature = "tokio-io")]
impl<R: tokio::io::AsyncRead> tokio::io::AsyncRead for NormalizingReader<R> {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut tokio::io::ReadBuf<'_>,
    ) -> Poll<std::io::Result<()>> {
        self.project().inner.poll_read(cx, buf)
    }
}
