use std::{mem::MaybeUninit, ptr};

use memchr::memchr2;

use crate::{
    formats::{NormalizeChunk, NormalizeChunkResult},
    types::{CR, LF},
    Error, Result,
};

/// CRLF normalization format implementation.
///
/// Will convert all line endings that are not CRLF (i.e. LF or CR alone) into CRLF.
pub struct CRLF;

impl NormalizeChunk for CRLF {
    fn max_output_size_for_chunk(
        chunk_size: usize,
        _preceded_by_cr: bool,
        is_last_chunk: bool,
    ) -> usize {
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
        chunk_size * 2 + usize::from(is_last_chunk)
    }

    fn normalize_chunk(
        input: &[u8],
        output: &mut [MaybeUninit<u8>],
        preceded_by_cr: bool,
        is_last_chunk: bool,
    ) -> Result<NormalizeChunkResult> {
        let output_required =
            Self::max_output_size_for_chunk(input.len(), preceded_by_cr, is_last_chunk);
        if output.len() < output_required {
            return Err(Error::OutputBufferTooSmall {
                required: output_required,
            });
        }

        if input.is_empty() && !is_last_chunk {
            // Special case: empty input and not last chunk
            return Ok(NormalizeChunkResult::new(0, preceded_by_cr));
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
            output[0] = MaybeUninit::new(LF);
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
                                output.as_mut_ptr().add(write_pos).cast::<u8>(),
                                bytes_now,
                            );
                            *output.get_unchecked_mut(write_pos + bytes_now) = MaybeUninit::new(CR);
                            *output.get_unchecked_mut(write_pos + bytes_now + 1) =
                                MaybeUninit::new(LF);
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
                                output.as_mut_ptr().add(write_pos).cast::<u8>(),
                                bytes_now,
                            );
                            if is_last_chunk {
                                *output.get_unchecked_mut(write_pos + bytes_now) =
                                    MaybeUninit::new(LF);
                            }
                        }
                        break Ok(NormalizeChunkResult::new(
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
                        output.as_mut_ptr().add(write_pos).cast::<u8>(),
                        bytes_now,
                    );
                }
                break Ok(NormalizeChunkResult::new(write_pos + bytes_now, false));
            }
        }
    }
}
