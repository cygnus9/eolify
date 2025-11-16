#![doc = include_str!("../README.md")]

pub mod core;
pub mod io;

#[cfg(feature = "futures-io")]
pub mod futures_io;

#[cfg(feature = "tokio")]
pub mod tokio;
