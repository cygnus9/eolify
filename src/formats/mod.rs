//! The `formats` module contains the core traits and types for normalization. The actual
//! formats (like CRLF) are implemented in submodules.

use std::mem::MaybeUninit;

use crate::{helpers::vec_to_uninit_mut, Result};

pub(crate) mod crlf;
pub(crate) mod lf;

/// Result returned by `normalize_chunk` describing how many bytes were
/// written and the state to pass to the next chunk.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NormalizeChunkResult<S: Sized> {
    output_len: usize,
    state: Option<S>,
}

impl<S> NormalizeChunkResult<S> {
    /// Construct a new `NormalizeChunkResult`.
    #[must_use]
    pub fn new(output_len: usize, state: Option<S>) -> Self {
        Self { output_len, state }
    }

    /// Returns the number of bytes written into the output buffer for the
    /// last processed chunk.
    #[must_use]
    pub fn output_len(&self) -> usize {
        self.output_len
    }

    /// The state to pass to the next invocation of `normalize_chunk`.
    /// Its meaning is defined by the normalizer; LF and CRLF use a boolean
    /// indicating whether the input ended with an unpaired `\r`.
    #[must_use]
    pub fn state(&self) -> Option<&S> {
        self.state.as_ref()
    }
}

/// This is the core trait that defines how to normalize a chunk data to a specific format.
///
/// Consumers will typically not use this trait directly, but rather the higher-level
/// `Normalize` trait. Custom normalizers are supported, but implementing this
/// trait requires an explicit safety audit.
///
/// # Safety
///
/// Implementations must uphold all of the following for every input, output
/// buffer, state, and `is_last_chunk` value accepted by these safe methods:
///
/// - On success, `output_len()` must be at most `output.len()`, and every byte
///   in `output[..output_len()]` must be initialized.
/// - Never deinitialize a byte of the output buffer that was already initialized,
///   including on error or panic. Consumers may supply initialized byte buffers
///   and reuse them after failure or unwinding.
/// - Never read uninitialized output bytes or access either buffer out of bounds.
///   Insufficient space must produce an error rather than an out-of-bounds write.
/// - With `state = None` and `is_last_chunk = true`, valid UTF-8 input must
///   produce valid UTF-8 output on success. The blanket `Normalize::normalize_str`
///   implementation relies on this when constructing a `String`.
///
/// `max_output_size_for_chunk` must provide enough space for the corresponding
/// call to succeed. The allocation and I/O consumers rely on the initialized
/// prefix and initialization-preservation guarantees above, not just the size
/// estimate. A size estimate alone cannot establish memory safety.
///
/// Safe implementations are rejected:
///
/// ```compile_fail,E0200
/// use std::mem::MaybeUninit;
/// use eolify::{NormalizeChunk, NormalizeChunkResult, Result};
/// struct Bad;
/// impl NormalizeChunk for Bad {
///     type State = ();
///     fn max_output_size_for_chunk(_: usize, _: Option<&()>, _: bool) -> usize { 1 }
///     fn normalize_chunk(_: &[u8], output: &mut [MaybeUninit<u8>],
///         _: Option<&()>, _: bool) -> Result<NormalizeChunkResult<()>> {
///         Ok(NormalizeChunkResult::new(output.len() + 1, None))
///     }
/// }
/// ```
pub unsafe trait NormalizeChunk {
    type State: Clone + Sized;

    /// Normalize a single chunk of input to the required format into the provided `output` buffer.
    ///
    /// Parameters:
    /// - `input`: bytes to normalize
    /// - `output`: destination buffer.
    /// - `state`: the previous result's state, or `None` for the first chunk.
    /// - `is_last_chunk`: set to `true` if this is the final chunk of the stream.
    ///
    /// Returns a `NormalizeChunkResult` on success which tells how many bytes were
    /// written and the state for the next chunk.
    ///
    /// # Errors
    ///
    /// Returns `Err(crate::Error::OutputBufferTooSmall { required })` if `output`
    /// is too small to hold the  expansion of `input`. The implementation is expected
    /// (but not required) to calculate the worst-case size without actually processing
    /// the input.
    fn normalize_chunk(
        input: &[u8],
        output: &mut [MaybeUninit<u8>],
        state: Option<&Self::State>,
        is_last_chunk: bool,
    ) -> Result<NormalizeChunkResult<Self::State>>;

    /// Returns the worst-case required output buffer size for the given `chunk_size`.
    #[must_use]
    fn max_output_size_for_chunk(
        chunk_size: usize,
        state: Option<&Self::State>,
        is_last_chunk: bool,
    ) -> usize;
}

/// This is the trait that consumers will typically use to normalize vectors or
/// string slices to a specific format.
pub trait Normalize {
    /// Normalize the entire input buffer and return a newly allocated `Vec<u8>` with the result.
    #[must_use]
    fn normalize(input: &[u8]) -> Vec<u8>;

    /// Normalize the entire input string and return a newly allocated `String` with the result.
    #[must_use]
    fn normalize_str(input: &str) -> String;
}

impl<N: NormalizeChunk> Normalize for N {
    fn normalize(input: &[u8]) -> Vec<u8> {
        let mut output =
            Vec::with_capacity(Self::max_output_size_for_chunk(input.len(), None, true));
        // SAFETY: output has length zero, so there are no initialized vector
        // elements to preserve. NormalizeChunk initializes the reported prefix.
        let status =
            Self::normalize_chunk(input, unsafe { vec_to_uninit_mut(&mut output) }, None, true)
                .unwrap_or_else(|err| unreachable!("{err} (should be impossible)",));

        // SAFETY: NormalizeChunk's unsafe contract guarantees an initialized
        // prefix of this length within the supplied allocation.
        unsafe {
            output.set_len(status.output_len());
        }

        output
    }

    fn normalize_str(input: &str) -> String {
        // SAFETY: NormalizeChunk's unsafe contract requires UTF-8 preservation
        // for valid input with no initial state and is_last_chunk = true.
        unsafe { String::from_utf8_unchecked(Self::normalize(input.as_bytes())) }
    }
}
