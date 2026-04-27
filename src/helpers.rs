use std::mem::MaybeUninit;

/// Returns a mutable `MaybeUninit<u8>` view over the full spare allocation of a `Vec<u8>`.
///
/// The returned slice length is `vec.capacity()`, not `vec.len()`. This is intended for
/// low-level [`crate::NormalizeChunk`] callers that want to let a normalizer write directly into
/// a vector's spare capacity and then set the vector length to the reported initialized length.
///
/// # Safety contract for callers
///
/// This function itself is safe to call, but it is usually paired with an unsafe
/// [`Vec::set_len`] call after normalization. Only call `set_len` with a byte count returned by a
/// successful [`crate::NormalizeChunk::normalize_chunk`] call using this buffer. Reading from the
/// vector's spare capacity, or setting the vector length beyond the number of initialized bytes,
/// is undefined behavior.
///
/// # Example
///
/// ```rust
/// use eolify::{helpers::vec_to_uninit_mut, NormalizeChunk, CRLF};
///
/// let input = b"a\nb";
/// let mut output = Vec::with_capacity(CRLF::max_output_size_for_chunk(input.len(), None, true));
///
/// let status = CRLF::normalize_chunk(input, vec_to_uninit_mut(&mut output), None, true)?;
/// unsafe {
///     output.set_len(status.output_len());
/// }
///
/// assert_eq!(output, b"a\r\nb");
/// # Ok::<(), eolify::Error>(())
/// ```
pub fn vec_to_uninit_mut(vec: &mut Vec<u8>) -> &mut [MaybeUninit<u8>] {
    unsafe {
        std::slice::from_raw_parts_mut(vec.as_mut_ptr().cast::<MaybeUninit<u8>>(), vec.capacity())
    }
}

/// Returns a mutable `MaybeUninit<u8>` view over an initialized byte slice.
///
/// This is primarily useful when calling [`crate::NormalizeChunk::normalize_chunk`] with an
/// already-sized output buffer, such as the fixed buffers used by this crate's I/O wrappers.
/// Unlike [`vec_to_uninit_mut`], the returned slice covers only the existing slice length because
/// a `&mut [u8]` has no spare capacity.
///
/// The bytes may be overwritten by the normalizer. After a successful call, only the first
/// [`crate::NormalizeChunkResult::output_len`] bytes are part of the normalized output.
///
/// # Example
///
/// ```rust
/// use eolify::{helpers::slice_to_uninit_mut, NormalizeChunk, LF};
///
/// let input = b"a\r\nb";
/// let mut output = [0; 8];
///
/// let status = LF::normalize_chunk(input, slice_to_uninit_mut(&mut output), None, true)?;
///
/// assert_eq!(&output[..status.output_len()], b"a\nb");
/// # Ok::<(), eolify::Error>(())
/// ```
pub fn slice_to_uninit_mut(slice: &mut [u8]) -> &mut [MaybeUninit<u8>] {
    unsafe { &mut *(std::ptr::from_mut::<[u8]>(slice) as *mut [MaybeUninit<u8>]) }
}
