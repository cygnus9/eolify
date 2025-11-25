//! The `formats` module contains the core traits and types for normalization. The actual
//! formats (like CRLF) are implemented in submodules.

use crate::{Error, Result};

pub(crate) mod crlf;
pub(crate) mod lf;

/// Result returned by `normalize_chunk` describing how many bytes were
/// written and whether the chunk ended with a `\r`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NormalizeChunkResult {
    output_len: usize,
    ended_with_cr: bool,
}

impl NormalizeChunkResult {
    /// Construct a new `NormalizeChunkResult`.
    #[must_use]
    pub fn new(output_len: usize, ended_with_cr: bool) -> Self {
        Self {
            output_len,
            ended_with_cr,
        }
    }

    /// Returns the number of bytes written into the output buffer for the
    /// last processed chunk.
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

/// This is the core trait that defines how to normalize data to a specific format.
///
/// Implementors must provide the `normalize_chunk` method which performs the actual
/// normalization logic. All other methods have default implementations based on
/// `normalize_chunk`. They can be overridden if appropriate.
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
    /// is too small to hold the  expansion of `input`. The implementation is expected
    /// (but not required) to calculate the worst-case size without actually processing
    /// the input.
    fn normalize_chunk(
        input: &[u8],
        output: &mut [u8],
        preceded_by_cr: bool,
        is_last_chunk: bool,
    ) -> Result<NormalizeChunkResult>;

    /// Returns the required output buffer size for the given input buffer.
    ///
    /// The default implementation calls `normalize_chunk` with an empty output buffer
    /// to determine the worst-case required size.
    #[must_use]
    fn output_size_for(input: &[u8]) -> usize {
        let Err(Error::OutputBufferTooSmall { required }) =
            Self::normalize_chunk(input, &mut [], false, true)
        else {
            unreachable!("output buffer should be too small when passing empty buffer");
        };
        required
    }

    /// Normalize the entire input buffer and return a newly allocated `Vec<u8>` with the result.
    #[must_use]
    fn normalize(input: &[u8]) -> Vec<u8> {
        let mut output = vec![0u8; Self::output_size_for(input)];
        let status = Self::normalize_chunk(input, &mut output, false, true)
            .unwrap_or_else(|err| unreachable!("{err} (should be impossible)",));
        output.truncate(status.output_len());
        output
    }

    /// Normalize the entire input string and return a newly allocated `String` with the result.
    #[must_use]
    fn normalize_str(input: &str) -> String {
        // SAFETY: normalize returns valid UTF-8 when given valid UTF-8 input because we only
        // insert ASCII CR/LF bytes.
        unsafe { String::from_utf8_unchecked(Self::normalize(input.as_bytes())) }
    }
}
