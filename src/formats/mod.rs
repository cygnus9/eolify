use std::string::FromUtf8Error;

use crate::{Error, Result};

pub(crate) mod crlf;

/// Status returned by `normalize_chunk` describing how many bytes were
/// written and whether the chunk ended with a `\r`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NormalizeChunkResult {
    output_len: usize,
    ended_with_cr: bool,
}

impl NormalizeChunkResult {
    #[must_use]
    pub(crate) fn new(output_len: usize, ended_with_cr: bool) -> Self {
        Self {
            output_len,
            ended_with_cr,
        }
    }

    /// Number of bytes written into the output buffer for this chunk.
    #[must_use]
    pub fn output_len(&self) -> usize {
        self.output_len
    }

    /// Whether the input ended with an unpaired `\r`.
    ///
    /// If `true`, the next invocation of `normalize_chunk` should have `preceded_by_cr`
    /// set to `true` to properly handle a possible leading `\n`.
    #[must_use]
    pub fn ended_with_cr(&self) -> bool {
        self.ended_with_cr
    }
}

pub trait Normalize {
    /// Normalize a single chunk of input to the required format into the provided `output` buffer.
    ///
    /// Parameters:
    /// - `input`: bytes to normalize
    /// - `output`: destination buffer.
    /// - `preceded_by_cr`: set to `true` if the previous chunk ended with a `\r`.
    /// - `is_last_chunk`: set to `true` if this is the final chunk of the stream.
    ///
    /// Returns a `NormalizeChunkResult` on success which tells how many bytes were
    /// written and whether the chunk ended with a dangling `\r`.
    ///
    /// # Errors
    ///
    /// Returns `Err(crate::Error::OutputBufferTooSmall { required })` if `output`
    /// is too small to hold the worst-case expansion of `input`.
    fn normalize_chunk(
        input: &[u8],
        output: &mut [u8],
        preceded_by_cr: bool,
        is_last_chunk: bool,
    ) -> Result<NormalizeChunkResult>;

    #[must_use]
    fn output_size_for(input: &[u8]) -> usize {
        let Err(Error::OutputBufferTooSmall { required }) =
            Self::normalize_chunk(input, &mut [], false, true)
        else {
            unreachable!("output buffer should be too small when passing empty buffer");
        };
        required
    }

    #[must_use]
    fn normalize(input: &[u8]) -> Vec<u8> {
        let mut output = vec![0u8; Self::output_size_for(input)];
        let status = Self::normalize_chunk(input, &mut output, false, true)
            .unwrap_or_else(|err| unreachable!("{err} (should be impossible)",));
        output.truncate(status.output_len());
        output
    }

    #[must_use]
    fn normalize_str(input: &str) -> String {
        // normalize returns valid UTF-8 when given valid UTF-8 input because we only
        // insert ASCII CR/LF bytes.
        String::from_utf8(Self::normalize(input.as_bytes())).unwrap_or_else(
            |FromUtf8Error { .. }| {
                unreachable!("normalize produced invalid UTF-8 (should be impossible)")
            },
        )
    }
}
