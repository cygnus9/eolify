#![doc = include_str!("../README.md")]

mod types;
pub use types::{Error, Result};

mod formats;
pub use formats::{crlf::CRLF, lf::LF, Normalize, NormalizeChunkResult};

mod wrappers;
pub use wrappers::io::{IoExt, ReadExt, WriteExt};

#[cfg(feature = "futures-io")]
pub use wrappers::futures_io::{FuturesIoAsyncReadExt, FuturesIoAsyncWriteExt, FuturesIoExt};

#[cfg(feature = "tokio")]
pub use wrappers::tokio::{TokioAsyncReadExt, TokioAsyncWriteExt, TokioExt};
