use std::{
    pin::{pin, Pin},
    task::{Context, Poll},
};

use futures_io::AsyncRead;
use pin_project_lite::pin_project;

use crate::{
    async_io::read::{PollRead, ReadBuffer},
    core::Spec,
};

pin_project! {
    pub struct NormalizingReader<R, S> {
        #[pin]
        reader: R,
        buf: ReadBuffer<S>,
    }
}

impl<R, S: Spec> NormalizingReader<R, S> {
    pub fn new(reader: R) -> Self {
        Self::with_size(reader, 8192)
    }

    pub fn with_size(reader: R, buf_size: usize) -> Self {
        Self {
            reader,
            buf: ReadBuffer::new(buf_size),
        }
    }

    pub fn into_inner(self) -> R {
        self.reader
    }
}

struct FuturesIoReader<R: futures_io::AsyncRead>(R);

impl<R: futures_io::AsyncRead + Unpin> PollRead for FuturesIoReader<R> {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut [u8],
    ) -> Poll<std::io::Result<usize>> {
        let this = self.get_mut();
        Pin::new(&mut this.0).poll_read(cx, buf)
    }
}

impl<R: AsyncRead, S: Spec> AsyncRead for NormalizingReader<R, S> {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut [u8],
    ) -> Poll<std::io::Result<usize>> {
        let this = self.project();
        let reader = pin!(FuturesIoReader(this.reader));
        this.buf.poll_read(cx, reader, buf)
    }
}

pub mod crlf;
