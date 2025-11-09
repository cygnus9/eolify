//! I/O adapters for CRLF normalization.
//!
//! This module provides `std::io::Read` and `std::io::Write` adapters that
//! normalizes incoming bytes to CRLF on-the-fly. Useful when you want to
//! stream-normalize data without materializing the whole input or output.

pub type NormalizingReader<R> = super::NormalizingReader<R, crate::core::crlf::Spec>;
pub type NormalizingWriter<W> = super::NormalizingWriter<W, crate::core::crlf::Spec>;
