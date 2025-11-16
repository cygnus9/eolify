use std::{
    future::Future,
    pin::{pin, Pin},
    task::{Context, Poll},
};

use futures_io::{AsyncRead, AsyncWrite};
use pin_project_lite::pin_project;

use crate::core::{
    async_io::{AsyncReadCompat, AsyncWriteCompat, ReadBuffer, WriteBuffer},
    Spec,
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

impl<R: futures_io::AsyncRead + Unpin> AsyncReadCompat for FuturesIoReader<R> {
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

pin_project! {
    pub struct NormalizingWriter<W, S> {
        #[pin]
        writer: W,
        buf: WriteBuffer<S>,
    }
}

impl<W, S: Spec> NormalizingWriter<W, S> {
    pub fn new(writer: W) -> Self {
        Self::with_size(writer, 8192)
    }

    pub fn with_size(writer: W, buf_size: usize) -> Self {
        Self {
            writer,
            buf: WriteBuffer::new(buf_size),
        }
    }
}

impl<W: AsyncWrite + Unpin, S: Spec> NormalizingWriter<W, S> {
    pub fn finish(self) -> impl Future<Output = std::io::Result<W>> {
        Finisher {
            writer: Some(self.writer),
            buf: self.buf,
        }
    }
}

pin_project! {
struct Finisher<W, S> {
    #[pin]
    writer: Option<W>,
    buf: WriteBuffer<S>,
}
}

impl<W: AsyncWrite + Unpin, S: Spec> Future for Finisher<W, S> {
    type Output = std::io::Result<W>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let mut this = self.project();

        let Some(writer) = this.writer.as_mut().get_mut() else {
            panic!("polled after completion");
        };

        let writer = pin!(FuturesIoWriter(writer));
        match this.buf.poll_flush(cx, writer, true) {
            Poll::Ready(Ok(())) => {}
            Poll::Ready(Err(e)) => return Poll::Ready(Err(e)),
            Poll::Pending => return Poll::Pending,
        }

        Poll::Ready(Ok(this.writer.as_mut().get_mut().take().unwrap()))
    }
}

struct FuturesIoWriter<W: AsyncWrite>(W);

impl<W: AsyncWrite + Unpin> AsyncWriteCompat for FuturesIoWriter<W> {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<std::io::Result<usize>> {
        let this = self.get_mut();
        Pin::new(&mut this.0).poll_write(cx, buf)
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        let this = self.get_mut();
        Pin::new(&mut this.0).poll_flush(cx)
    }

    fn poll_finish(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        let this = self.get_mut();
        Pin::new(&mut this.0).poll_close(cx)
    }
}

impl<W: AsyncWrite, S: Spec> AsyncWrite for NormalizingWriter<W, S> {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<std::result::Result<usize, std::io::Error>> {
        let this = self.project();
        let writer = pin!(FuturesIoWriter(this.writer));
        this.buf.poll_write(cx, writer, buf)
    }

    fn poll_flush(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<std::result::Result<(), std::io::Error>> {
        let this = self.project();
        let writer = pin!(FuturesIoWriter(this.writer));
        this.buf.poll_flush(cx, writer, false)
    }

    fn poll_close(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        let this = self.project();
        let writer = pin!(FuturesIoWriter(this.writer));
        this.buf.poll_finish(cx, writer)
    }
}

pub mod crlf;
