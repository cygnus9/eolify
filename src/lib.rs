#![doc = include_str!("../README.md")]

use std::{fmt, string::FromUtf8Error};

const CR: u8 = b'\r';
const LF: u8 = b'\n';

/// Status returned by `normalize_chunk` describing how many bytes were
/// written and whether the chunk ended with a `\r`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NormalizeChunkStatus {
    output_len: usize,
    ended_with_cr: bool,
}

impl NormalizeChunkStatus {
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

/// Error type for eolify operations.
///
/// Currently this enum has a single variant: it indicates that a caller-supplied
/// output buffer was too small to hold the worst-case expansion of the input.
/// Streaming helpers in this crate size their internal buffers so that this
/// error cannot occur; the fallible chunk API returns this error to allow
/// callers to allocate appropriately.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// The provided output buffer was too small.
    ///
    /// `required` is the number of bytes necessary to hold the worst-case
    /// expansion of the input (worst-case depends on the use normalization).
    OutputBufferTooSmall { required: usize },
}

impl Error {
    #[must_use]
    pub fn required_size(&self) -> usize {
        match self {
            Error::OutputBufferTooSmall { required } => *required,
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::OutputBufferTooSmall { required } => {
                write!(f, "output buffer is too small; required {required} bytes")
            }
        }
    }
}

impl std::error::Error for Error {}

pub type Result<T> = std::result::Result<T, Error>;

#[cfg(any(feature = "futures-io", feature = "tokio-io"))]
pub mod async_io;
pub mod crlf;
pub mod io;

type NormalizeChunkFn = fn(&[u8], &mut [u8], bool, bool) -> Result<NormalizeChunkStatus>;
type NormalizeFn = fn(&[u8]) -> Vec<u8>;

fn normalize(input: &[u8], fn_normalize_chunk: NormalizeChunkFn) -> Vec<u8> {
    let Err(crate::Error::OutputBufferTooSmall { required }) =
        fn_normalize_chunk(input, &mut [], false, true)
    else {
        unreachable!("output buffer should be too small when passing empty buffer");
    };
    let mut output = vec![0u8; required];
    let status = fn_normalize_chunk(input, &mut output, false, true)
        .unwrap_or_else(|err| unreachable!("{err} (should be impossible)",));
    output.truncate(status.output_len());
    output
}

fn normalize_str(input: &str, fn_normalize: NormalizeFn) -> String {
    // normalize returns valid UTF-8 when given valid UTF-8 input because we only
    // insert ASCII CR/LF bytes.
    String::from_utf8(fn_normalize(input.as_bytes())).unwrap_or_else(|FromUtf8Error { .. }| {
        unreachable!("normalize produced invalid UTF-8 (should be impossible)")
    })
}
