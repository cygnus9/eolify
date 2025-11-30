use std::{
    marker::PhantomData,
    pin::Pin,
    task::{Context, Poll},
};

use crate::{helpers::slice_to_uninit_mut, NormalizeChunk};

pub trait AsyncReadCompat {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut [u8],
    ) -> Poll<std::io::Result<usize>>;
}

pub struct ReadBuffer<N: NormalizeChunk> {
    _phantom: PhantomData<N>,
    input_buf: Box<[u8]>,
    output_buf: Box<[u8]>,
    output_pos: usize,
    output_size: usize,
    state: Option<N::State>,
    end_of_stream: bool,
}

impl<N: NormalizeChunk> ReadBuffer<N> {
    #[must_use]
    pub fn new(buf_size: usize) -> Self {
        let input_buf = vec![0; buf_size].into_boxed_slice();
        let required = N::max_output_size_for_chunk(buf_size, None, false);
        Self {
            _phantom: PhantomData,
            input_buf,
            output_buf: vec![0; required].into_boxed_slice(),
            output_pos: 0,
            output_size: 0,
            state: None,
            end_of_stream: false,
        }
    }

    pub fn poll_read<R: AsyncReadCompat>(
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

    pub fn poll_fill_buf<R: AsyncReadCompat>(
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

        let status = N::normalize_chunk(
            &self.input_buf[..bytes_read],
            slice_to_uninit_mut(&mut self.output_buf),
            self.state.as_ref(),
            is_last_chunk,
        )
        .map_err(std::io::Error::other)?;

        self.output_size = status.output_len();
        self.state = status.state().cloned();
        Poll::Ready(Ok(()))
    }
}

pub trait AsyncWriteCompat {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<std::io::Result<usize>>;

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<()>>;

    fn poll_finish(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<()>>;
}

pub struct WriteBuffer<N: NormalizeChunk> {
    _phantom: std::marker::PhantomData<N>,
    input_buf: Box<[u8]>,
    output_buf: Box<[u8]>,
    input_pos: usize,
    output_pos: usize,
    output_size: usize,
    state: Option<N::State>,
    stream_state: State,
}

pub enum State {
    Writing,
    Finishing,
    Finished,
}

impl<N: NormalizeChunk> WriteBuffer<N> {
    #[must_use]
    pub fn new(buf_size: usize) -> Self {
        let input_buf = vec![0; buf_size].into_boxed_slice();
        let required = N::max_output_size_for_chunk(buf_size, None, false);
        Self {
            _phantom: PhantomData,
            input_buf,
            output_buf: vec![0; required].into_boxed_slice(),
            input_pos: 0,
            output_pos: 0,
            output_size: 0,
            state: None,
            stream_state: State::Writing,
        }
    }

    pub fn poll_write<W: AsyncWriteCompat>(
        &mut self,
        cx: &mut Context<'_>,
        mut inner: Pin<&mut W>,
        buf: &[u8],
    ) -> Poll<std::io::Result<usize>> {
        let mut source_buf = buf;
        let mut total_bytes = 0;

        loop {
            if self.output_pos < self.output_size {
                // There is still data to write
                match inner
                    .as_mut()
                    .poll_write(cx, &self.output_buf[self.output_pos..self.output_size])
                {
                    Poll::Ready(Ok(n)) => {
                        self.output_pos += n;
                    }
                    other => return other,
                }
            } else {
                // Output buffer is empty, refill it
                self.output_pos = 0;
                self.output_size = 0;

                let bytes_now = source_buf.len().min(self.input_buf.len() - self.input_pos);
                total_bytes += bytes_now;

                self.input_buf[self.input_pos..self.input_pos + bytes_now]
                    .copy_from_slice(&source_buf[..bytes_now]);
                self.input_pos += bytes_now;
                source_buf = &source_buf[bytes_now..];

                if self.input_pos < self.input_buf.len() {
                    // Not enough data yet to process a full chunk.
                    return Poll::Ready(Ok(total_bytes));
                }

                let status = N::normalize_chunk(
                    &self.input_buf[..self.input_pos],
                    slice_to_uninit_mut(&mut self.output_buf),
                    self.state.as_ref(),
                    false,
                )
                .map_err(std::io::Error::other)?;

                self.state = status.state().cloned();
                self.output_size = status.output_len();
                self.input_pos = 0;
            }
        }
    }

    pub fn poll_flush<W: AsyncWriteCompat>(
        &mut self,
        cx: &mut Context<'_>,
        mut inner: Pin<&mut W>,
        finish: bool,
    ) -> Poll<std::io::Result<()>> {
        loop {
            if self.output_size == 0 {
                // Output buffer is empty, try to fill it
                let status = N::normalize_chunk(
                    &self.input_buf[..self.input_pos],
                    slice_to_uninit_mut(&mut self.output_buf),
                    self.state.as_ref(),
                    finish,
                )
                .map_err(std::io::Error::other)?;

                self.state = status.state().cloned();
                self.output_size = status.output_len();
                self.input_pos = 0;

                if self.output_size == 0 {
                    // Nothing more to write
                    return Poll::Ready(Ok(()));
                }
            } else if self.output_pos < self.output_size {
                // There is still data to write
                match inner
                    .as_mut()
                    .poll_write(cx, &self.output_buf[self.output_pos..self.output_size])
                {
                    Poll::Ready(Ok(n)) => {
                        self.output_pos += n;
                    }
                    Poll::Ready(Err(e)) => return Poll::Ready(Err(e)),
                    Poll::Pending => return Poll::Pending,
                }
            } else {
                // All data is written, flush the underlying writer
                match inner.as_mut().poll_flush(cx) {
                    Poll::Ready(Ok(())) => {}
                    other => return other,
                }
                self.output_pos = 0;
                self.output_size = 0;
            }
        }
    }

    pub fn poll_finish<W: AsyncWriteCompat>(
        &mut self,
        cx: &mut Context<'_>,
        mut inner: Pin<&mut W>,
    ) -> Poll<std::io::Result<()>> {
        if let State::Writing = self.stream_state {
            match self.poll_flush(cx, inner.as_mut(), true) {
                Poll::Ready(Ok(())) => {}
                other => return other,
            }
            self.stream_state = State::Finishing;
        }

        if let State::Finishing = self.stream_state {
            match inner.poll_finish(cx) {
                Poll::Ready(Ok(())) => {
                    self.stream_state = State::Finished;
                }
                other => return other,
            }
        }

        Poll::Ready(Ok(()))
    }
}
