use std::{
    pin::{pin, Pin},
    task::{Context, Poll},
};

use pin_project_lite::pin_project;

use crate::NormalizeChunkFn;

pub mod crlf;

pin_project! {
    struct NormalizingReader<R> {
        #[pin]
        reader: R,
        buf: ReadBuffer,
    }
}

impl<R> NormalizingReader<R> {
    fn new(reader: R, fn_normalize_chunk: NormalizeChunkFn) -> Self {
        Self::with_size(reader, fn_normalize_chunk, 8192)
    }

    fn with_size(reader: R, fn_normalize_chunk: NormalizeChunkFn, buf_size: usize) -> Self {
        Self {
            reader,
            buf: ReadBuffer::new(fn_normalize_chunk, buf_size),
        }
    }

    fn into_inner(self) -> R {
        self.reader
    }
}

trait PollRead {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut [u8],
    ) -> Poll<std::io::Result<usize>>;
}

#[cfg(feature = "futures-io")]
struct FuturesIoReader<R: futures_io::AsyncRead>(R);

#[cfg(feature = "tokio-io")]
struct TokioIoReader<R: tokio::io::AsyncRead>(R);

#[cfg(feature = "futures-io")]
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

#[cfg(feature = "tokio-io")]
impl<R: tokio::io::AsyncRead + Unpin> PollRead for TokioIoReader<R> {
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

struct ReadBuffer {
    fn_normalize_chunk: NormalizeChunkFn,
    input_buf: Box<[u8]>,
    output_buf: Box<[u8]>,
    output_pos: usize,
    output_size: usize,
    last_was_cr: bool,
    end_of_stream: bool,
}

impl ReadBuffer {
    fn new(fn_normalize_chunk: NormalizeChunkFn, buf_size: usize) -> Self {
        let input_buf = vec![0; buf_size].into_boxed_slice();
        let Err(crate::Error::OutputBufferTooSmall { required }) =
            fn_normalize_chunk(&input_buf, &mut [], false, false)
        else {
            unreachable!("output buffer should be too small when passing empty buffer");
        };
        Self {
            fn_normalize_chunk,
            input_buf,
            output_buf: vec![0; required].into_boxed_slice(),
            output_pos: 0,
            output_size: 0,
            last_was_cr: false,
            end_of_stream: false,
        }
    }

    fn poll_read<R: PollRead>(
        &mut self,
        cx: &mut Context<'_>,
        inner: Pin<&mut R>,
        buf: &mut [u8],
    ) -> Poll<std::io::Result<usize>> {
        if self.output_pos >= self.output_size {
            match self.poll_fill_buf(cx, inner) {
                Poll::Ready(Ok(())) => {}
                Poll::Ready(Err(e)) => return Poll::Ready(Err(e)),
                Poll::Pending => return Poll::Pending,
            }
        }

        if self.output_size == 0 {
            return Poll::Ready(Ok(0));
        }

        let bytes_now = buf.len().min(self.output_size - self.output_pos);
        buf[..bytes_now]
            .copy_from_slice(&self.output_buf[self.output_pos..self.output_pos + bytes_now]);
        self.output_pos += bytes_now;
        Poll::Ready(Ok(bytes_now))
    }

    fn poll_fill_buf<R: PollRead>(
        &mut self,
        cx: &mut Context<'_>,
        inner: Pin<&mut R>,
    ) -> Poll<std::io::Result<()>> {
        self.output_pos = 0;
        self.output_size = 0;

        if self.end_of_stream {
            return Poll::Ready(Ok(()));
        }

        let bytes_read = match inner.poll_read(cx, &mut self.input_buf) {
            Poll::Ready(Ok(n)) => n,
            Poll::Ready(Err(e)) => return Poll::Ready(Err(e)),
            Poll::Pending => return Poll::Pending,
        };
        let is_last_chunk = if bytes_read == 0 {
            self.end_of_stream = true;
            true
        } else {
            false
        };

        let status = (self.fn_normalize_chunk)(
            &self.input_buf[..bytes_read],
            &mut self.output_buf,
            self.last_was_cr,
            is_last_chunk,
        )
        .map_err(std::io::Error::other)?;

        self.output_size = status.output_len();
        self.last_was_cr = status.ended_with_cr();
        Poll::Ready(Ok(()))
    }
}

#[cfg(feature = "futures-io")]
impl<R: futures_io::AsyncRead> futures_io::AsyncRead for NormalizingReader<R> {
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

#[cfg(feature = "tokio-io")]
impl<R: tokio::io::AsyncRead> tokio::io::AsyncRead for NormalizingReader<R> {
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
