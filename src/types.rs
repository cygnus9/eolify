use core::fmt;

pub const CR: u8 = b'\r';
pub const LF: u8 = b'\n';

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
