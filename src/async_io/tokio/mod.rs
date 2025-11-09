use std::{
    pin::{pin, Pin},
    task::{Context, Poll},
};

use pin_project_lite::pin_project;
use tokio::io::AsyncRead;

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

struct TokioIoReader<R: AsyncRead>(R);

impl<R: AsyncRead + Unpin> PollRead for TokioIoReader<R> {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut [u8],
    ) -> Poll<std::io::Result<usize>> {
        let this = self.get_mut();
        let mut read_buf = tokio::io::ReadBuf::new(buf);
        match Pin::new(&mut this.0).poll_read(cx, &mut read_buf) {
            Poll::Ready(Ok(())) => Poll::Ready(Ok(read_buf.filled().len())),
            Poll::Ready(Err(e)) => Poll::Ready(Err(e)),
            Poll::Pending => Poll::Pending,
        }
    }
}

impl<R: AsyncRead, S: Spec> AsyncRead for NormalizingReader<R, S> {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut tokio::io::ReadBuf<'_>,
    ) -> Poll<std::io::Result<()>> {
        let this = self.project();
        let reader = pin!(TokioIoReader(this.reader));
        match this.buf.poll_read(cx, reader, buf.initialize_unfilled()) {
            Poll::Ready(Ok(n)) => {
                buf.advance(n);
                Poll::Ready(Ok(()))
            }
            Poll::Ready(Err(e)) => Poll::Ready(Err(e)),
            Poll::Pending => Poll::Pending,
        }
    }
}

pub mod crlf;
