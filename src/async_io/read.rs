use std::{
    marker::PhantomData,
    pin::Pin,
    task::{Context, Poll},
};

use crate::core::{Error, Spec};

pub(crate) trait PollRead {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut [u8],
    ) -> Poll<std::io::Result<usize>>;
}

pub(crate) struct ReadBuffer<S> {
    _phantom: PhantomData<S>,
    input_buf: Box<[u8]>,
    output_buf: Box<[u8]>,
    output_pos: usize,
    output_size: usize,
    last_was_cr: bool,
    end_of_stream: bool,
}

impl<S: Spec> ReadBuffer<S> {
    pub(crate) fn new(buf_size: usize) -> Self {
        let input_buf = vec![0; buf_size].into_boxed_slice();
        let Err(Error::OutputBufferTooSmall { required }) =
            S::FN_NORMALIZE_CHUNK(&input_buf, &mut [], false, false)
        else {
            unreachable!("output buffer should be too small when passing empty buffer");
        };
        Self {
            _phantom: PhantomData,
            input_buf,
            output_buf: vec![0; required].into_boxed_slice(),
            output_pos: 0,
            output_size: 0,
            last_was_cr: false,
            end_of_stream: false,
        }
    }

    pub(crate) fn poll_read<R: PollRead>(
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

    pub(crate) fn poll_fill_buf<R: PollRead>(
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

        let status = S::FN_NORMALIZE_CHUNK(
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
