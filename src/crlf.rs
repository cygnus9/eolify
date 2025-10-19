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
use memchr::memchr2;

use crate::NormalizeChunkStatus;

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
) -> crate::Result<NormalizeChunkStatus> {
    // Worst case: every byte in input needs conversion, plus one extra byte for the
    // trailing CR from last chunk.
    let output_required = input.len() * 2 + 1;
    if output.len() < output_required {
        return Err(crate::Error::OutputBufferTooSmall {
            required: output_required,
        });
    }

    let mut scan_pos = 0;
    let mut read_pos = 0;
    let mut write_pos = 0;

    if input.first() == Some(&b'\n') && preceded_by_cr {
        // We found:
        // - a LF preceeded by a CR from the previous chunk
        scan_pos = 1;
    } else if preceded_by_cr {
        // We found:
        // - not a LF preceeded by a CR from the previous chunk, or
        // - empty input preceeded by a CR from the previous chunk
        output[0] = b'\n';
        write_pos = 1;
    }

    loop {
        if let Some(i) = memchr2(b'\r', b'\n', &input[scan_pos..]).map(|i| i + scan_pos) {
            if input[i] == b'\n' || (i + 1 < input.len() && input[i + 1] != b'\n') {
                // We found:
                // - a LF not preceeded by a CR, or
                // - a CR not followed by a LF and not at the last position
                let bytes_now = i - read_pos;
                output[write_pos..write_pos + bytes_now]
                    .copy_from_slice(&input[read_pos..read_pos + bytes_now]);
                output[write_pos + bytes_now..write_pos + bytes_now + 2].copy_from_slice(b"\r\n");
                read_pos = i + 1;
                scan_pos = read_pos;
                write_pos += bytes_now + 2;
            } else if input[i] == b'\r' && i + 1 < input.len() {
                // We found:
                // - a CR followed by a LF
                // Intentionally don't copy now — advance scan_pos to skip the CRLF
                // so we'll include the CRLF in a later large bulk copy from read_pos.
                scan_pos = i + 2;
            } else if i + 1 == input.len() {
                // We found:
                // - a CR at the last position
                let bytes_now = input.len() - read_pos;
                output[write_pos..write_pos + bytes_now].copy_from_slice(&input[read_pos..]);
                if is_last_chunk {
                    // Last chunk: emit CRLF
                    output[write_pos + bytes_now] = b'\n';
                    break Ok(NormalizeChunkStatus {
                        output_len: write_pos + bytes_now + 1,
                        ended_with_cr: false,
                    });
                }
                break Ok(NormalizeChunkStatus {
                    output_len: write_pos + bytes_now,
                    ended_with_cr: true,
                });
            }
        } else {
            // We found:
            // - the end of the input
            let bytes_now = input.len() - read_pos;
            output[write_pos..write_pos + bytes_now].copy_from_slice(&input[read_pos..]);
            break Ok(NormalizeChunkStatus {
                output_len: write_pos + bytes_now,
                ended_with_cr: false,
            });
        }
    }
}

/// Normalize a whole byte slice to CRLF, returning an owned `Vec<u8>`.
///
/// This is a convenience wrapper that internally allocates a buffer large
/// enough to guarantee the chunk API cannot fail.
#[must_use]
pub fn normalize(input: &[u8]) -> Vec<u8> {
    crate::normalize(input, normalize_chunk)
}

/// Normalize a UTF-8 string to CRLF and return an owned String.
///
/// Safe because normalization only inserts ASCII CR/LF bytes and therefore
/// preserves UTF-8 validity.
#[must_use]
pub fn normalize_str(input: &str) -> String {
    crate::normalize_str(input, normalize)
}
