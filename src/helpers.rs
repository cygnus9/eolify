#[cfg(feature = "alloc")]
use alloc::vec::Vec;
use core::{mem::MaybeUninit, ptr, slice};

/// Returns a mutable `MaybeUninit<u8>` view over the full allocation of a `Vec<u8>`.
///
/// The returned slice length is `vec.capacity()`, not `vec.len()`. This is intended for
/// low-level [`crate::NormalizeChunk`] callers that want to let a normalizer write directly into
/// a vector's spare capacity and then set the vector length to the reported initialized length.
///
/// # Safety
///
/// The caller must keep all bytes in the original `vec[..vec.len()]` initialized,
/// including when unwinding. Writing `MaybeUninit::uninit()` into an existing
/// element would violate this requirement. Calling a conforming
/// [`crate::NormalizeChunk`] implementation preserves initialization.
///
/// A later [`Vec::set_len`] must remain within capacity and expose only initialized
/// bytes. A successful normalizer call initializes its reported output prefix.
///
/// ```compile_fail,E0133
/// let mut bytes = vec![1u8];
/// eolify::helpers::vec_to_uninit_mut(&mut bytes)[0] = std::mem::MaybeUninit::uninit();
/// ```
///
/// # Example
///
/// ```rust
/// use eolify::{helpers::vec_to_uninit_mut, NormalizeChunk, CRLF};
///
/// let input = b"a\nb";
/// let mut output = Vec::with_capacity(CRLF::max_output_size_for_chunk(input.len(), None, true));
///
/// let status = CRLF::normalize_chunk(input, unsafe { vec_to_uninit_mut(&mut output) }, None, true)?;
/// unsafe {
///     output.set_len(status.output_len());
/// }
///
/// assert_eq!(output, b"a\r\nb");
/// # Ok::<(), eolify::Error>(())
/// ```
#[cfg(feature = "alloc")]
pub unsafe fn vec_to_uninit_mut(vec: &mut Vec<u8>) -> &mut [MaybeUninit<u8>] {
    // SAFETY: The pointer covers the vector allocation; the caller guarantees
    // that initialized elements remain initialized throughout this borrow.
    unsafe { slice::from_raw_parts_mut(vec.as_mut_ptr().cast::<MaybeUninit<u8>>(), vec.capacity()) }
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
/// # Safety
///
/// The caller must keep the entire original slice initialized, including on
/// error or panic. A conforming [`crate::NormalizeChunk`] implementation never
/// deinitializes existing bytes, so it is safe to pass this view to one.
///
/// ```compile_fail,E0133
/// let mut bytes = [1u8];
/// eolify::helpers::slice_to_uninit_mut(&mut bytes)[0] = std::mem::MaybeUninit::uninit();
/// ```
///
/// # Example
///
/// ```rust
/// use eolify::{helpers::slice_to_uninit_mut, NormalizeChunk, LF};
///
/// let input = b"a\r\nb";
/// let mut output = [0; 8];
///
/// let status = LF::normalize_chunk(input, unsafe { slice_to_uninit_mut(&mut output) }, None, true)?;
///
/// assert_eq!(&output[..status.output_len()], b"a\nb");
/// # Ok::<(), eolify::Error>(())
/// ```
pub unsafe fn slice_to_uninit_mut(slice: &mut [u8]) -> &mut [MaybeUninit<u8>] {
    // SAFETY: u8 and MaybeUninit<u8> have identical layout. The caller guarantees
    // the original slice stays initialized throughout this borrow.
    unsafe { &mut *(ptr::from_mut::<[u8]>(slice) as *mut [MaybeUninit<u8>]) }
}
