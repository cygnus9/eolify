# Changelog

All notable changes to this project will be documented in this file.

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

See the Upgrade section in the README for guidance on porting code that
used the old chunk API to the new `NormalizeChunk` trait.
