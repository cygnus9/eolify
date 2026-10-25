# Changelog

All notable changes to this project will be documented in this file.

## [0.5.0] - 2026-10-25

### no_std support

- Add `std` and `alloc` features. The chunk normalization API is available
  without either feature; whole-buffer `Vec`/`String` APIs require `alloc`.
- Support `no_std` builds and implement `core::error::Error` (MSRV 1.81).

### Safety and compatibility

- Make `NormalizeChunk` an unsafe trait with explicit initialization, bounds,
  initialization-preservation (including errors/panics), and whole-buffer UTF-8
  guarantees. Custom normalizers remain supported, but require an audited
  `unsafe impl`; safe implementations are no longer accepted.
- Make both public buffer-view helpers unsafe: a mutable `MaybeUninit` view of
  existing bytes can otherwise deinitialize values accessible through safe APIs.
  Existing low-level callers must preserve initialization and use an unsafe block.
- Audit synchronous and asynchronous consumers, document built-in implementation
  safety, fix state documentation, and add compile-fail and initialization tests.
  Built-in LF/CRLF high-level APIs remain safe to call.

## [0.4.0] - 2025-12-20

### Notable (user-facing) changes

- Refactored the chunked normalization API: the previous `Normalize` trait
  was split into `NormalizeChunk` (core chunk-processing trait) and a
  convenience `Normalize` impl for whole-buffer APIs. This changes the
  trait names and how consumers integrate chunk-based normalization.

- The `normalize_chunk` API changed significantly:
  - Output buffers are now `MaybeUninit<u8>` for zero-initialization avoidance.
  - Implementations receive an optional `state` parameter (an associated
    `State` type) instead of a `preceded_by_cr: bool`, and return a
    `NormalizeChunkResult<State>` containing the next-state.
  - A new `max_output_size_for_chunk(chunk_size, state, is_last_chunk)` helper
    replaces the old sizing API.

- `NormalizeChunkResult` is now generic over a `State` and exposes the
  resulting state instead of a simple boolean `ended_with_cr` flag.

- Public exports in `lib.rs` and several format implementations were updated
  to use the new traits and signatures. If you implemented custom formats
  or used the low-level chunk API, you will need to adapt to these new
  types and function signatures.

### Other changes

- Performance improvements and additional benchmarks were added.

### Migration notes

When upgrading from 0.3.x to 0.4, update low-level chunk API callers:

- Replace the old chunk-processing `Normalize` trait with `NormalizeChunk`;
  `Normalize` now provides allocating whole-buffer convenience methods.
- Supply a `MaybeUninit<u8>` output buffer and `Option<&Self::State>` instead
  of `preceded_by_cr: bool`. The associated `State` carries information such
  as whether the previous input chunk ended with an unpaired CR.
- Retrieve the next chunk state with `NormalizeChunkResult::state()` instead
  of the former `ended_with_cr()` boolean.
- Size each output buffer with
  `max_output_size_for_chunk(chunk_size, state, is_last_chunk)` before calling
  `normalize_chunk`. Pass the same state and final-chunk flag to both calls.

For example, a current whole-buffer chunk call is:

```rust
use eolify::{NormalizeChunk, LF};

fn main() -> eolify::Result<()> {
    let input = b"one\r\ntwo\r";
    let capacity = LF::max_output_size_for_chunk(input.len(), None, true);
    let mut output = Vec::<u8>::with_capacity(capacity);
    let status = LF::normalize_chunk(input, output.spare_capacity_mut(), None, true)?;
    // SAFETY: the normalizer initialized this prefix within the allocation.
    unsafe { output.set_len(status.output_len()); }
    assert_eq!(output, b"one\ntwo\n");
    Ok(())
}
```

High-level `normalize` and `normalize_str` callers need no changes for this
refactor. For the additional custom-normalizer and buffer-helper safety
requirements, see **Safety and compatibility** under Unreleased above and the
[README safety guidance](README.md#custom-normalizers-and-buffer-safety).
