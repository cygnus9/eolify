#![doc = include_str!("../README.md")]
#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(feature = "alloc")]
extern crate alloc;

mod types;

pub use types::{Error, Result};

mod analyze;
#[cfg(feature = "std")]
pub use analyze::analyze_reader;
pub use analyze::{analyze_chunk, AnalyzeState, LineEndingStats};

mod formats;
#[cfg(feature = "alloc")]
pub use formats::Normalize;
pub use formats::{crlf::CRLF, lf::LF, NormalizeChunk, NormalizeChunkResult};

#[cfg(feature = "std")]
mod wrappers;
#[cfg(feature = "std")]
pub use wrappers::io::{IoExt, ReadExt, WriteExt};

#[cfg(feature = "futures-io")]
pub use wrappers::futures_io::{FuturesIoAsyncReadExt, FuturesIoAsyncWriteExt, FuturesIoExt};

#[cfg(feature = "tokio")]
pub use wrappers::tokio::{TokioAsyncReadExt, TokioAsyncWriteExt, TokioExt};

pub mod helpers;
