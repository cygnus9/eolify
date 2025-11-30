//! The `io` module provides wrappers for standard I/O `Read` and `Write`
//! traits to perform newline normalization on-the-fly.

use std::{
    io::{Read, Write},
    marker::PhantomData,
};

use crate::{helpers::slice_to_uninit_mut, NormalizeChunk};

/// A `std::io::Read` wrapper and implementation that normalizes newlines on-the-fly.
pub struct Reader<R, N: NormalizeChunk> {
    _phantom: PhantomData<N>,
    inner: R,
    input_buf: Box<[u8]>,
    output_buf: Box<[u8]>,
    output_pos: usize,
    output_size: usize,
    state: Option<N::State>,
    end_of_stream: bool,
}

impl<R: Read, N: NormalizeChunk> Reader<R, N> {
    pub fn new(reader: R, buf_size: usize) -> Self {
        let input_buf = vec![0; buf_size].into_boxed_slice();
        let required = N::max_output_size_for_chunk(buf_size, None, false);
        Self {
            _phantom: PhantomData,
            inner: reader,
            input_buf,
            output_buf: vec![0; required].into_boxed_slice(),
            output_pos: 0,
            output_size: 0,
            state: None,
            end_of_stream: false,
        }
    }

    fn fill_buf(&mut self) -> std::io::Result<()> {
        self.output_pos = 0;
        self.output_size = 0;

        if self.end_of_stream {
            return Ok(());
        }

        let bytes_read = self.inner.read(&mut self.input_buf)?;
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
        Ok(())
    }

    pub fn into_inner(self) -> R {
        self.inner
    }
}

impl<R: Read, N: NormalizeChunk> Read for Reader<R, N> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        if self.output_pos >= self.output_size {
            self.fill_buf()?;
        }
        if self.output_size == 0 {
            return Ok(0);
        }

        let bytes_now = buf.len().min(self.output_size - self.output_pos);
        buf[..bytes_now]
            .copy_from_slice(&self.output_buf[self.output_pos..self.output_pos + bytes_now]);
        self.output_pos += bytes_now;
        Ok(bytes_now)
    }
}

/// A `std::io::Write` wrapper and implementation that normalizes newlines on-the-fly.
pub struct Writer<W, S: NormalizeChunk> {
    _phantom: PhantomData<S>,
    inner: W,
    input_buf: Box<[u8]>,
    output_buf: Box<[u8]>,
    input_pos: usize,
    state: Option<S::State>,
}

impl<W: Write, N: NormalizeChunk> Writer<W, N> {
    pub fn new(inner: W, buf_size: usize) -> Self {
        let input_buf = vec![0; buf_size].into_boxed_slice();
        let required = N::max_output_size_for_chunk(buf_size, None, false);
        Self {
            _phantom: PhantomData,
            inner,
            input_buf,
            output_buf: vec![0; required].into_boxed_slice(),
            input_pos: 0,
            state: None,
        }
    }

    pub fn finish(self) -> std::io::Result<W> {
        let mut this = self;
        // Finalize any remaining input
        let status = N::normalize_chunk(
            &this.input_buf[..this.input_pos],
            slice_to_uninit_mut(&mut this.output_buf),
            this.state.as_ref(),
            true, // this is the last chunk
        )
        .map_err(std::io::Error::other)?;

        this.inner
            .write_all(&this.output_buf[..status.output_len()])?;
        Ok(this.inner)
    }
}

impl<W: Write, N: NormalizeChunk> Write for Writer<W, N> {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let mut source_buf = buf;
        let mut total_bytes = 0;

        while total_bytes < buf.len() {
            let bytes_now = source_buf.len().min(self.input_buf.len() - self.input_pos);
            total_bytes += bytes_now;

            self.input_buf[self.input_pos..self.input_pos + bytes_now]
                .copy_from_slice(&source_buf[..bytes_now]);
            self.input_pos += bytes_now;
            source_buf = &source_buf[bytes_now..];

            if self.input_pos < self.input_buf.len() {
                // Not enough data yet to process a full chunk.
                return Ok(total_bytes);
            }

            let status = N::normalize_chunk(
                &self.input_buf,
                slice_to_uninit_mut(&mut self.output_buf),
                self.state.as_ref(),
                false,
            )
            .map_err(std::io::Error::other)?;

            self.inner
                .write_all(&self.output_buf[..status.output_len()])?;
            self.state = status.state().cloned();
            self.input_pos = 0;
        }
        Ok(total_bytes)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        let status = N::normalize_chunk(
            &self.input_buf[..self.input_pos],
            slice_to_uninit_mut(&mut self.output_buf),
            self.state.as_ref(),
            false, // flush is not neccesarily the end of stream
        )
        .map_err(std::io::Error::other)?;

        if status.output_len() > 0 {
            self.inner
                .write_all(&self.output_buf[..status.output_len()])?;
            self.state = status.state().cloned();
            self.input_pos = 0;
        }
        self.inner.flush()
    }
}

/// Extension trait to provide convenient methods on `Normalize` for `std::io::Read`
/// and `std::io::Write`.
pub trait IoExt
where
    Self: Sized + NormalizeChunk,
{
    /// Wrap a reader with a newline-normalizing `Reader`.
    fn wrap_reader<R: Read>(reader: R) -> Reader<R, Self> {
        Self::wrap_reader_with_buffer_size(reader, 8192)
    }

    /// Wrap a reader with a newline-normalizing `Reader` and specify the internal buffer size.
    fn wrap_reader_with_buffer_size<R: Read>(reader: R, buf_size: usize) -> Reader<R, Self>;

    /// Wrap a writer with a newline-normalizing `Writer`.
    fn wrap_writer<W: Write>(writer: W) -> Writer<W, Self> {
        Self::wrap_writer_with_buffer_size(writer, 8192)
    }

    /// Wrap a writer with a newline-normalizing `Writer` and specify the internal buffer size.
    fn wrap_writer_with_buffer_size<W: Write>(writer: W, buf_size: usize) -> Writer<W, Self>;
}

impl<N: NormalizeChunk> IoExt for N {
    fn wrap_reader_with_buffer_size<R: Read>(reader: R, buf_size: usize) -> Reader<R, Self> {
        Reader::<R, Self>::new(reader, buf_size)
    }

    fn wrap_writer_with_buffer_size<W: Write>(writer: W, buf_size: usize) -> Writer<W, Self> {
        Writer::<W, Self>::new(writer, buf_size)
    }
}

/// Extension trait to provide convenient methods on `std::io::Read`.
pub trait ReadExt {
    /// Wrap the reader with a newline-normalizing `Reader`.
    fn normalize_newlines<N: NormalizeChunk>(self, _: N) -> Reader<Self, N>
    where
        Self: Sized;
}

impl<R: Read> ReadExt for R {
    fn normalize_newlines<N: NormalizeChunk>(self, _: N) -> Reader<Self, N>
    where
        Self: Sized,
    {
        N::wrap_reader(self)
    }
}

/// Extension trait to provide convenient methods on `std::io::Write`.
pub trait WriteExt {
    /// Wrap the writer with a newline-normalizing `Writer`.
    fn normalize_newlines<N: NormalizeChunk>(self, _: N) -> Writer<Self, N>
    where
        Self: Sized;
}

impl<W: Write> WriteExt for W {
    fn normalize_newlines<N: NormalizeChunk>(self, _: N) -> Writer<Self, N>
    where
        Self: Sized,
    {
        N::wrap_writer(self)
    }
}
