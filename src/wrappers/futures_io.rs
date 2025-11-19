use std::{
    future::Future,
    pin::{pin, Pin},
    task::{Context, Poll},
};

use futures_io::{AsyncRead, AsyncWrite};
use pin_project_lite::pin_project;

use crate::{
    wrappers::async_core::{AsyncReadCompat, AsyncWriteCompat, ReadBuffer, WriteBuffer},
    Normalize,
};

pin_project! {
    pub struct AsyncReader<R, N> {
        #[pin]
        reader: R,
        buf: ReadBuffer<N>,
    }
}

impl<R, N: Normalize> AsyncReader<R, N> {
    pub fn new(reader: R, buf_size: usize) -> Self {
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

impl<R: AsyncRead, N: Normalize> AsyncRead for AsyncReader<R, N> {
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
    pub struct AsyncWriter<W, N> {
        #[pin]
        writer: W,
        buf: WriteBuffer<N>,
    }
}

impl<W, N: Normalize> AsyncWriter<W, N> {
    pub fn new(writer: W, buf_size: usize) -> Self {
        Self {
            writer,
            buf: WriteBuffer::new(buf_size),
        }
    }
}

impl<W: AsyncWrite + Unpin, N: Normalize> AsyncWriter<W, N> {
    pub fn finish(self) -> impl Future<Output = std::io::Result<W>> {
        Finisher {
            writer: Some(self.writer),
            buf: self.buf,
        }
    }
}

pin_project! {
struct Finisher<W, N> {
    #[pin]
    writer: Option<W>,
    buf: WriteBuffer<N>,
}
}

impl<W: AsyncWrite + Unpin, N: Normalize> Future for Finisher<W, N> {
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

impl<W: AsyncWrite, N: Normalize> AsyncWrite for AsyncWriter<W, N> {
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

pub trait FuturesIoExt
where
    Self: Sized,
{
    fn wrap_async_reader<R: AsyncRead>(reader: R) -> AsyncReader<R, Self> {
        Self::wrap_async_reader_with_buffer_size(reader, 8192)
    }

    fn wrap_async_reader_with_buffer_size<R: AsyncRead>(
        reader: R,
        buf_size: usize,
    ) -> AsyncReader<R, Self>;

    fn wrap_async_writer<W: AsyncWrite>(writer: W) -> AsyncWriter<W, Self> {
        Self::wrap_async_writer_with_buffer_size(writer, 8192)
    }

    fn wrap_async_writer_with_buffer_size<W: AsyncWrite>(
        writer: W,
        buf_size: usize,
    ) -> AsyncWriter<W, Self>;
}

impl<N: Normalize> FuturesIoExt for N {
    fn wrap_async_reader_with_buffer_size<R: AsyncRead>(
        reader: R,
        buf_size: usize,
    ) -> AsyncReader<R, Self> {
        AsyncReader::<R, Self>::new(reader, buf_size)
    }

    fn wrap_async_writer_with_buffer_size<W: AsyncWrite>(
        writer: W,
        buf_size: usize,
    ) -> AsyncWriter<W, Self> {
        AsyncWriter::<W, Self>::new(writer, buf_size)
    }
}

pub trait FuturesIoAsyncReadExt {
    fn normalize_newlines<N: Normalize>(self, _: N) -> AsyncReader<Self, N>
    where
        Self: Sized;
}

impl<R: AsyncRead> FuturesIoAsyncReadExt for R {
    fn normalize_newlines<N: Normalize>(self, _: N) -> AsyncReader<Self, N>
    where
        Self: Sized,
    {
        N::wrap_async_reader(self)
    }
}

pub trait FuturesIoAsyncWriteExt {
    fn normalize_newlines<N: Normalize>(self, _: N) -> AsyncWriter<Self, N>
    where
        Self: Sized;
}

impl<W: AsyncWrite> FuturesIoAsyncWriteExt for W {
    fn normalize_newlines<N: Normalize>(self, _: N) -> AsyncWriter<Self, N>
    where
        Self: Sized,
    {
        N::wrap_async_writer(self)
    }
}
