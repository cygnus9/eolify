use std::io::Read;

use crate::NormalizeChunkFn;

pub mod crlf;

struct NormalizingReader<R> {
    fn_normalize_chunk: NormalizeChunkFn,
    inner: R,
    input_buf: Box<[u8]>,
    output_buf: Box<[u8]>,
    output_pos: usize,
    output_size: usize,
    last_was_cr: bool,
    end_of_stream: bool,
}

impl<R: Read> NormalizingReader<R> {
    pub fn new(inner: R, fn_normalize_chunk: NormalizeChunkFn) -> Self {
        Self::with_size(inner, fn_normalize_chunk, 8192)
    }

    pub fn with_size(inner: R, fn_normalize_chunk: NormalizeChunkFn, buf_size: usize) -> Self {
        let input_buf = vec![0; buf_size].into_boxed_slice();
        let Err(crate::Error::OutputBufferTooSmall { required }) =
            fn_normalize_chunk(&input_buf, &mut [], false, false)
        else {
            unreachable!("output buffer should be too small when passing empty buffer");
        };
        Self {
            fn_normalize_chunk,
            inner,
            input_buf,
            output_buf: vec![0; required].into_boxed_slice(),
            output_pos: 0,
            output_size: 0,
            last_was_cr: false,
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

        let status = (self.fn_normalize_chunk)(
            &self.input_buf[..bytes_read],
            &mut self.output_buf,
            self.last_was_cr,
            is_last_chunk,
        )
        .map_err(std::io::Error::other)?;

        self.output_size = status.output_len();
        self.last_was_cr = status.ended_with_cr();
        Ok(())
    }
}

impl<R: Read> Read for NormalizingReader<R> {
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
