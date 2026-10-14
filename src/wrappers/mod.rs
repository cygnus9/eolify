#[cfg(feature = "std")]
pub(crate) mod io;

#[cfg(all(feature = "std", any(feature = "futures-io", feature = "tokio")))]
pub mod async_core;

#[cfg(all(feature = "std", feature = "futures-io"))]
pub(crate) mod futures_io;

#[cfg(all(feature = "std", feature = "tokio"))]
pub(crate) mod tokio;
