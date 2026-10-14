# eolify  
**High-performance line ending normalization for Rust**  

[![crates.io](https://img.shields.io/crates/v/eolify.svg?color=blue)](https://crates.io/crates/eolify)

`eolify` is a lightweight, allocation-conscious library for normalizing end-of-line (EOL) sequences in large text streams or buffers. It’s designed for high-throughput processing pipelines, data ingestion systems, and cross-platform tooling where consistency and efficiency matter.  

## Features  
- Fast and memory-efficient — optimized for bulk text processing.  
- Normalizes EOLs to a consistent format: CRLF (`\r\n`) or LF (`\n`).
- Minimal dependencies — ideal for embedding in performance-critical code.  
- Handles mixed endings (`\n`, `\r\n`, `\r`) gracefully.  
- Supports:  
  - Chunk-based API (buffer slices)  
  - Synchronous implementations of `Read` / `Write`  
  - Asynchronous implementations of `AsyncRead` / `AsyncWrite` (both `futures_io` and `tokio` supported).

## Current status  
- Normalization to CRLF (`\r\n`) is implemented.
- Normalization to LF (`\n`) is implemented.

## Usage  

### Simple string normalization  
```rust
use eolify::{CRLF, Normalize};

let text = "one\nline\r\ntwo\rthree";
let normalized = CRLF::normalize_str(text);
assert_eq!(normalized, "one\r\nline\r\ntwo\r\nthree");
println!("{}", normalized);
```

### Synchronous I/O reader / writer
```rust,ignore
use std::fs::File;
use std::io::{BufWriter, Write};
use eolify::{CRLF, ReadExt};

fn normalize_file_sync(input_path: &str, output_path: &str) -> std::io::Result<()> {
    let infile = File::open(input_path)?;
    let mut reader = infile.normalize_newlines(CRLF);

    let outfile = File::create(output_path)?;
    let mut writer = BufWriter::new(outfile);

    std::io::copy(&mut reader, &mut writer)?;
    writer.flush()?;
    Ok(())
}
```

## Why use eolify?

Working with large text files or streams (logs, ingestion pipelines, cross-platform toolchains) often involves inconsistent line endings (LF, CRLF, CR). Instead of ad-hoc `.replace()` or loading everything into memory, eolify offers a streaming, allocation-conscious approach so you can normalize while reading or writing, without multiple allocations or buffering the entire file.

## Getting started

Add to your Cargo.toml:

```toml
[dependencies]
eolify = { version = "0.4", features = ["tokio"] }

# Alternatively enable the `futures-io` async wrappers instead of `tokio`:
# eolify = { version = "0.4", features = ["futures-io"] }
```

Then either call the high-level string routines (for small chunks) or use the I/O wrappers for streaming use-cases.

### Asynchronous I/O

Enable the `tokio` feature (see Cargo snippet above) and use the `TokioAsyncReadExt` / `TokioAsyncWriteExt` helpers:

```nocompile
use tokio::fs::File;
use tokio::io::{AsyncWriteExt, BufWriter};
use eolify::{CRLF, TokioAsyncReadExt};

async fn normalize_file_async(input_path: &str, output_path: &str) -> std::io::Result<()> {
  let infile = File::open(input_path).await?;
  let mut reader = infile.normalize_newlines(CRLF);

  let outfile = File::create(output_path).await?;
  let mut writer = BufWriter::new(outfile);

  tokio::io::copy(&mut reader, &mut writer).await?;
  writer.shutdown().await?;
  Ok(())
}
```

For `futures-io`, enable the `futures-io` feature and use the equivalent
`FuturesIoAsyncReadExt` / `FuturesIoAsyncWriteExt` helpers.

## `no_std` and Cargo features

By default, `eolify` enables the `std` feature, which includes the standard I/O
adapters, reader analysis, allocation based APIs, and `memchr` runtime CPU
detection. The `std` feature enables `alloc`.

For chunk normalization without `std` or a heap allocator, disable default
features:

```toml
eolify = { version = "0.4", default-features = false }
```

Enable `alloc` to use whole-buffer `Normalize` methods that return `Vec` or
`String`; a `no_std` application must provide an allocator for those methods.

```toml
eolify = { version = "0.4", default-features = false, features = ["alloc"] }
```

The Tokio and `futures-io` adapters require `std`. The crate requires Rust 1.81
or newer to implement `core::error::Error` for its error type.

## License

MIT or Apache-2.0, at your option.

## Custom normalizers and buffer safety

Custom normalizers remain supported. `NormalizeChunk` is an unsafe trait:
implement it with `unsafe impl` only after auditing every method against its
documented safety contract. Successful calls must report an initialized prefix
within the output buffer; existing initialized bytes must stay initialized even
on error or panic. Whole-buffer normalization must preserve valid UTF-8 so that
`Normalize::normalize_str` can safely construct a `String`.

This is a source compatibility change for downstream custom implementations.
Built-in `LF`/`CRLF` normalization and I/O extension methods remain safe to call.
The public `helpers::vec_to_uninit_mut` and `helpers::slice_to_uninit_mut`
conversions now require an unsafe block: keep existing bytes initialized for the
entire borrow, including unwinding. Passing the view directly to a conforming
normalizer satisfies that obligation. For a vector with existing contents,
`Vec::spare_capacity_mut` is a safe alternative when only spare capacity is needed;
it starts after the current length rather than at the beginning of the allocation.

## Upgrade notes (0.3.x → 0.4.0)

If you are upgrading from `0.3.x` to `0.4.0` there are API changes you
need to address if you used the low-level chunked normalization API:

- The old `Normalize` chunk trait was refactored into `NormalizeChunk` and
  a convenience `Normalize` impl now exists for whole-buffer operations.
- `normalize_chunk` now takes an output buffer of `MaybeUninit<u8>` and an
  optional `state: Option<&Self::State>` instead of `preceded_by_cr: bool`.
  Implementations should use the associated `State` type to track any
  carried state (e.g. whether the previous chunk ended with `\r`).
- The result type `NormalizeChunkResult` now returns `state()` to retrieve
  the next chunk state (previously `ended_with_cr()` boolean).
- Use `max_output_size_for_chunk(chunk_size, state, is_last_chunk)` to
  allocate output buffers with proper capacity before calling
  `normalize_chunk`.

Example migration pattern (pseudo-Rust):

```nocompile
// old (0.3.x)
let mut out = vec![0u8; input.len()];
let status = LF::normalize_chunk(input, &mut out, preceded_by_cr, true)?;

// new (0.4.0)
let mut out = Vec::with_capacity(Self::max_output_size_for_chunk(input.len(), None, true));
let status = LF::normalize_chunk(input, vec_to_uninit_mut(&mut out), None, true)?;
unsafe { out.set_len(status.output_len()); }
let state = status.state();
```

If you only used the higher-level `normalize` or `normalize_str` helpers,
no changes are required.
