//! CRLF normalization utilities.
//!
//! This module provides a small, allocation-conscious API for normalizing
//! end-of-line sequences to CRLF (`\r\n`). It exposes a chunk-oriented
//! primitive suitable for streaming scenarios and convenience helpers for
//! whole-buffer usage.
//!
//! The chunk API is high-performance and intended for callers that control
//! buffer allocation and want to avoid intermediate copies. The convenience
//! helpers (`normalize` / `normalize_str`) allocate an internal buffer sized
//! to guarantee `normalize_chunk` cannot fail.
use std::ptr;

use memchr::memchr2;

use crate::core::{Error, NormalizeChunkStatus, Result, CR, LF};

/// Normalize a single chunk of input to CRLF into the provided `output` buffer.
///
/// Parameters:
/// - `input`: bytes to normalize
/// - `output`: destination buffer. Worst-case required size is `input.len() * 2 + 1`.
/// - `preceded_by_cr`: set to `true` if the previous chunk ended with a `\r`.
/// - `is_last_chunk`: set to `true` if this is the final chunk of the stream.
///
/// Returns a `NormalizeChunkStatus` on success which tells how many bytes were
/// written and whether the chunk ended with a dangling `\r`.
///
/// # Errors
///
/// Returns `Err(crate::Error::OutputBufferTooSmall { required })` if `output`
/// is too small to hold the worst-case expansion of `input`.
pub fn normalize_chunk(
    input: &[u8],
    output: &mut [u8],
    preceded_by_cr: bool,
    is_last_chunk: bool,
) -> Result<NormalizeChunkStatus> {
    // Worst case: every byte in input needs conversion, plus one extra byte for the
    // trailing CR from last chunk.
    //
    // Example worst-case scenarios:
    // - input: "\n\n\n", preceded_by_cr: false, is_last_chunk: any   -> output: "\r\n\r\n\r\n"    (2n)
    // - input: "\n\n\n", preceded_by_cr: true,  is_last_chunk: any   -> output:   "\n\r\n\r\n"    (2n - 1)
    // - input: "\r\r\r", preceded_by_cr: false, is_last_chunk: false -> output:   "\r\n\r\n\r"    (2n - 1)
    // - input: "\r\r\r", preceded_by_cr: true,  is_last_chunk: false -> output: "\n\r\n\r\n\r"    (2n)
    // - input: "\r\r\r", preceded_by_cr: false, is_last_chunk: true  -> output:   "\r\n\r\n\r\n"  (2n)
    // - input: "\r\r\r", preceded_by_cr: true,  is_last_chunk: true  -> output: "\n\r\n\r\n\r\n"  (2n + 1)
    //
    // So the only case where we need 2n + 1 bytes is when `preceded_by_cr` is true, the last byte is a `\r` and
    // `is_last_chunk` is true. In all other cases, 2n bytes is sufficient. However, for simpliucity we'll only
    // look at `is_last_chunk`. We could just require 2n + 1 bytes always, but that would be surprising for
    // callers that intuitively expect 2n to be sufficient in all cases, or at least when not processing the
    // last chunk.
    let output_required = input.len() * 2 + usize::from(is_last_chunk);
    if output.len() < output_required {
        return Err(Error::OutputBufferTooSmall {
            required: output_required,
        });
    }

    if input.is_empty() && !is_last_chunk {
        // Special case: empty input and not last chunk
        return Ok(NormalizeChunkStatus::new(0, preceded_by_cr));
    }

    let mut scan_pos = 0;
    let mut read_pos = 0;
    let mut write_pos = 0;

    if input.first() == Some(&LF) && preceded_by_cr {
        // We found:
        // - a LF preceeded by a CR from the previous chunk
        scan_pos = 1;
    } else if preceded_by_cr {
        // We found:
        // - not a LF preceeded by a CR from the previous chunk, or
        // - empty input preceeded by a CR from the previous chunk
        output[0] = LF;
        write_pos = 1;
    }

    loop {
        if let Some(i) = memchr2(CR, LF, &input[scan_pos..]).map(|i| i + scan_pos) {
            // SAFETY: i is in-bounds because it was found by memchr2.
            let c = unsafe { *input.get_unchecked(i) };
            match (c, input.get(i + 1).copied()) {
                (CR, Some(LF)) => {
                    // We found:
                    // - a CR followed by a LF
                    // Intentionally don't copy now — advance scan_pos to skip the CRLF
                    // so we'll include the CRLF in a later large bulk copy from read_pos.
                    scan_pos = i + 2;
                }
                (CR, Some(_)) | (LF, _) => {
                    // We found:
                    // - a LF not preceeded by a CR, or
                    // - a CR not followed by a LF and not at the last position
                    let bytes_now = i - read_pos;
                    // SAFETY: read_pos..i is in-bounds because i was found by memchr2 and we've
                    // established at the top that output is large enough for worst-case expansion.
                    unsafe {
                        ptr::copy_nonoverlapping(
                            input.as_ptr().add(read_pos),
                            output.as_mut_ptr().add(write_pos),
                            bytes_now,
                        );
                        *output.get_unchecked_mut(write_pos + bytes_now) = CR;
                        *output.get_unchecked_mut(write_pos + bytes_now + 1) = LF;
                    }
                    read_pos = i + 1;
                    scan_pos = read_pos;
                    write_pos += bytes_now + 2;
                }
                (CR, None) => {
                    // We found:
                    // - a CR at the last position
                    let bytes_now = input.len() - read_pos;
                    // SAFETY: read_pos..end is in-bounds because 0 <= read_pos <= end and we've
                    // established at the top that output is large enough for worst-case expansion.
                    unsafe {
                        ptr::copy_nonoverlapping(
                            input.as_ptr().add(read_pos),
                            output.as_mut_ptr().add(write_pos),
                            bytes_now,
                        );
                        if is_last_chunk {
                            *output.get_unchecked_mut(write_pos + bytes_now) = LF;
                        }
                    }
                    break Ok(NormalizeChunkStatus::new(
                        write_pos + bytes_now + usize::from(is_last_chunk),
                        !is_last_chunk,
                    ));
                }
                _ => unreachable!("unreachable pattern match case"),
            }
        } else {
            // We found:
            // - the end of the input
            let bytes_now = input.len() - read_pos;
            // SAFETY: read_pos..end is in-bounds because 0 <= read_pos <= end and we've
            // established at the top that output is large enough for worst-case expansion.
            unsafe {
                ptr::copy_nonoverlapping(
                    input.as_ptr().add(read_pos),
                    output.as_mut_ptr().add(write_pos),
                    bytes_now,
                );
            }
            break Ok(NormalizeChunkStatus::new(write_pos + bytes_now, false));
        }
    }
}

/// Normalize a whole byte slice to CRLF, returning an owned `Vec<u8>`.
///
/// This is a convenience wrapper that internally allocates a buffer large
/// enough to guarantee the chunk API cannot fail.
#[must_use]
pub fn normalize(input: &[u8]) -> Vec<u8> {
    crate::core::normalize(input, normalize_chunk)
}

/// Normalize a UTF-8 string to CRLF and return an owned String.
///
/// Safe because normalization only inserts ASCII CR/LF bytes and therefore
/// preserves UTF-8 validity.
#[must_use]
pub fn normalize_str(input: &str) -> String {
    crate::core::normalize_str(input, normalize)
}

pub struct Spec;
impl crate::core::Spec for Spec {
    const FN_NORMALIZE_CHUNK: crate::core::NormalizeChunkFn = normalize_chunk;
}
