use core::fmt;

pub const CR: u8 = b'\r';
pub const LF: u8 = b'\n';

/// Error type for normalize operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// The provided output buffer was too small.
    ///
    /// `required` is the number of bytes necessary to hold the worst-case
    /// expansion of the input (worst-case depends on the used normalization).
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

/// Result type alias for normalize operations.
pub type Result<T> = std::result::Result<T, Error>;
