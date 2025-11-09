#![doc = include_str!("../README.md")]

#[cfg(any(feature = "futures-io", feature = "tokio-io"))]
pub mod async_io;
pub mod core;
pub mod io;
