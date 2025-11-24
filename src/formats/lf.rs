use std::ptr;

use memchr::memchr;

use crate::{formats::NormalizeChunkResult, types, Normalize, Result};

/// LF normalization format implementation.
///
/// Will convert all line endings that are not LF (i.e. CRLF or CR alone) into LF.
pub struct LF;

impl Normalize for LF {
    fn normalize_chunk(
        input: &[u8],
        output: &mut [u8],
        preceded_by_cr: bool,
        is_last_chunk: bool,
    ) -> Result<NormalizeChunkResult> {
        let output_required = input.len();
        if output.len() < output_required {
            return Err(crate::Error::OutputBufferTooSmall {
                required: output_required,
            });
        }

        if input.is_empty() {
            // If this is the last chunk we're no longer preceded_by_cr, if
            // it's not than we return the input.
            return Ok(NormalizeChunkResult::new(
                0,
                preceded_by_cr && !is_last_chunk,
            ));
        }

        let mut scan_pos = 0;
        let mut read_pos = 0;
        let mut write_pos = 0;

        if input.first() == Some(&types::LF) && preceded_by_cr {
            // We found:
            // - a LF preceeded by a CR from the previous chunk
            // The LF was already written when that CR was detected so we can
            // just skipt this LF.
            scan_pos = 1;
            read_pos = 1;
        }

        loop {
            if let Some(i) = memchr(types::CR, &input[scan_pos..]).map(|i| i + scan_pos) {
                // SAFETY: i is in-bounds because it was found by memchr.
                let c = unsafe { *input.get_unchecked(i) };
                match (c, input.get(i + 1).copied()) {
                    (types::CR, Some(types::LF)) => {
                        // We found:
                        // - a CR followed by a LF
                        // Copy everything up to i, update scan_pos to skip the CRLF and
                        // update read_pos to only skip the CR.
                        let bytes_now = i - read_pos;
                        // SAFETY: read_pos..i is in-bounds because i was found by memchr1 and we've
                        // established at the top that output is large enough for worst-case expansion.
                        unsafe {
                            ptr::copy_nonoverlapping(
                                input.as_ptr().add(read_pos),
                                output.as_mut_ptr().add(write_pos),
                                bytes_now,
                            );
                        }
                        scan_pos = i + 2;
                        read_pos = i + 1;
                        write_pos += bytes_now;
                    }
                    (types::CR, next) => {
                        // We found:
                        // - a CR followed by anything but an LF
                        // - a CR at the last position
                        // Copy everything up to i, output an LF and and depending on whether next is_some
                        // update scan_pos, read_pos and write_pos or break with a result.
                        let bytes_now = i - read_pos;
                        // SAFETY: read_pos..i is in-bounds because i was found by memchr1 and we've
                        // established at the top that output is large enough for worst-case expansion.
                        unsafe {
                            ptr::copy_nonoverlapping(
                                input.as_ptr().add(read_pos),
                                output.as_mut_ptr().add(write_pos),
                                bytes_now,
                            );
                            *output.get_unchecked_mut(write_pos + bytes_now) = types::LF;
                        }
                        if next.is_none() {
                            break Ok(NormalizeChunkResult::new(
                                write_pos + bytes_now + 1,
                                !is_last_chunk,
                            ));
                        }
                        scan_pos = i + 1;
                        read_pos = i + 1;
                        write_pos += bytes_now + 1;
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
                break Ok(NormalizeChunkResult::new(write_pos + bytes_now, false));
            }
        }
    }
}
