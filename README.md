# Eolify — Streaming newline normalization for Rust

[![crates.io](https://img.shields.io/crates/v/eolify.svg?color=blue)](https://crates.io/crates/eolify)

Convert mixed line endings (LF, CRLF, and CR) to consistent LF or CRLF. Eolify
integrates with standard Rust I/O, Tokio, and futures-io without loading entire
files into memory. CR is recognized as an **input** line ending; the supported
**output** modes are `LF` (`\n`) and `CRLF` (`\r\n`) only.

- **Synchronous streaming:** wrap existing `Read` and `Write` implementations.
- **Asynchronous streaming:** use Tokio or futures-io readers and writers.
- **Stateful normalization:** handle newline sequences across chunk boundaries.
- **Bounded memory:** I/O wrappers process large files and streams using internal
  buffers whose size does not grow with the input.
- **In-memory normalization:** normalize strings and byte buffers in one call.
- **Low-level control:** process chunks in `no_std` environments without allocation.

Eolify uses `memchr` for efficient scanning, which can benefit from SIMD
acceleration on supported targets and configurations.

## Quick Start

With `eolify = "0.5"` in your dependencies, the default features support both examples.

### Streaming

Wrap an existing reader and copy normalized bytes into your destination:

```rust
# #[cfg(feature = "std")]
fn main() -> std::io::Result<()> {
    use eolify::{IoExt, LF};
    use std::io;

    let input = &b"one\r\ntwo\rthree\n"[..];
    let mut reader = LF::wrap_reader(input);
    let mut output = Vec::new();
    io::copy(&mut reader, &mut output)?;
    assert_eq!(output, b"one\ntwo\nthree\n");
    Ok(())
}
# #[cfg(not(feature = "std"))]
# fn main() {}
```

The byte slice and `Vec` keep this example self-contained. Replace them with a
file, socket, or other I/O source and destination to process a stream with bounded
normalization buffers.

### One-shot

```rust
# #[cfg(feature = "alloc")]
fn main() {
    use eolify::{Normalize, LF};

    let text = LF::normalize_str("one\r\ntwo\rthree\n");
    assert_eq!(text, "one\ntwo\nthree\n");
}
# #[cfg(not(feature = "alloc"))]
# fn main() {}
```

Use `CRLF` in place of `LF` in either example to select CRLF output.

## Synchronous I/O

`IoExt` provides `LF::wrap_reader()` and `LF::wrap_writer()` (also available on
`CRLF`). Reader wrappers implement `std::io::Read`, so the streaming Quick Start
works with an existing `File` or any other reader.

Wrap the destination instead when normalization belongs on the write side:

```rust
# #[cfg(feature = "std")]
fn main() -> std::io::Result<()> {
    use eolify::{IoExt, CRLF};
    use std::io::Write;

    let mut writer = CRLF::wrap_writer(Vec::new());
    writer.write_all(b"one\n")?;
    writer.write_all(b"two\r")?;
    let output = writer.finish()?;
    assert_eq!(output, b"one\r\ntwo\r\n");
    Ok(())
}
# #[cfg(not(feature = "std"))]
# fn main() {}
```

Call `finish()` to finalize buffered input, including a trailing CR, and recover
the inner writer. `flush()` alone does not mark the end of the stream. If the
inner writer is buffered, flush it after `finish()` as needed.

`ReadExt` and `WriteExt` also offer `source.normalize_newlines(LF)` and
`destination.normalize_newlines(CRLF)`. To choose an internal buffer size, use
`wrap_reader_with_buffer_size()` or `wrap_writer_with_buffer_size()` with a
nonzero size. See the [file conversion example](examples/sync_file_convert.rs).

## Asynchronous I/O

### Tokio

Enable Eolify's `tokio` feature and Tokio's `io-util`, `macros`, and `rt` features
for this example:

```rust
# #[cfg(feature = "tokio")]
#[tokio::main(flavor = "current_thread")]
async fn main() -> std::io::Result<()> {
    use eolify::{TokioExt, LF};
    use tokio::io::AsyncWriteExt;

    let mut reader = LF::wrap_async_reader(&b"one\r\ntwo\r"[..]);
    let mut output = Vec::new();
    tokio::io::copy(&mut reader, &mut output).await?;
    assert_eq!(output, b"one\ntwo\n");

    let mut written = Vec::new();
    let mut writer = LF::wrap_async_writer(&mut written);
    writer.write_all(b"one\r\ntwo\r").await?;
    writer.shutdown().await?;
    drop(writer);
    assert_eq!(written, b"one\ntwo\n");
    Ok(())
}
# #[cfg(not(feature = "tokio"))]
# fn main() {}
```

`shutdown().await` finalizes the normalizer and shuts down the inner writer.
Alternatively, `finish().await` finalizes and returns the inner writer without
shutting it down. Use Tokio's `fs` feature for files, as shown in the
[Tokio file conversion example](examples/tokio_file_convert.rs).

### futures-io

Enable Eolify's `futures-io` feature and add
`futures-util = { version = "0.3", features = ["io"] }` for the copy and write
helpers. Run this async function using your executor:

```rust
# #[cfg(feature = "futures-io")]
async fn normalize() -> std::io::Result<()> {
    use eolify::{FuturesIoExt, CRLF};
    use futures_util::io::{AsyncWriteExt, Cursor};

    let mut reader = CRLF::wrap_async_reader(Cursor::new(b"one\ntwo\r"));
    let mut output = Cursor::new(Vec::new());
    futures_util::io::copy(&mut reader, &mut output).await?;
    assert_eq!(output.into_inner(), b"one\r\ntwo\r\n");

    let mut written = Vec::new();
    let mut writer = CRLF::wrap_async_writer(Cursor::new(&mut written));
    writer.write_all(b"one\ntwo\r").await?;
    writer.close().await?;
    drop(writer);
    assert_eq!(written, b"one\r\ntwo\r\n");
    Ok(())
}
# fn main() {
#     #[cfg(feature = "futures-io")]
#     tokio::runtime::Builder::new_current_thread().build().unwrap()
#         .block_on(normalize()).unwrap();
# }
```

`close().await` finalizes the normalizer and closes the inner writer;
`finish().await` returns the finalized inner writer without closing it.

The respective `TokioAsyncReadExt` / `TokioAsyncWriteExt` and
`FuturesIoAsyncReadExt` / `FuturesIoAsyncWriteExt` traits also provide
`normalize_newlines()` on existing I/O values. Import the traits for your chosen
I/O ecosystem to avoid ambiguity when both async features are enabled.

## In-memory normalization

The `Normalize` trait provides `normalize_str(&str) -> String` and
`normalize(&[u8]) -> Vec<u8>`. These methods allocate the complete output and
require the `alloc` feature (included by default).

```rust
# #[cfg(feature = "alloc")]
fn main() {
    use eolify::{Normalize, CRLF};

    assert_eq!(CRLF::normalize_str("one\ntwo\r"), "one\r\ntwo\r\n");
    assert_eq!(CRLF::normalize(b"one\ntwo\r"), b"one\r\ntwo\r\n");
}
# #[cfg(not(feature = "alloc"))]
# fn main() {}
```

## Advanced usage

### Chunk API and `no_std`

[`NormalizeChunk`](https://docs.rs/eolify/latest/eolify/trait.NormalizeChunk.html)
works without `std` or allocation. Supply a `MaybeUninit<u8>` output buffer sized
using `max_output_size_for_chunk(input.len(), state, is_last_chunk)`. Pass the
previous result's `state()` to the next call and set `is_last_chunk` only at the
end of the stream. If the stream ends after a nonfinal chunk, finalize with an
empty input chunk. Only the first `output_len()` bytes of a successful result
are initialized output.

```rust
fn main() -> eolify::Result<()> {
    use core::mem::MaybeUninit;
    use eolify::{NormalizeChunk, LF};

    let mut output = [MaybeUninit::<u8>::uninit(); 16];
    let first = LF::normalize_chunk(b"one\r", &mut output, None, false)?;
    let second = LF::normalize_chunk(b"\ntwo\r", &mut output, first.state(), true)?;
    assert_eq!(first.output_len() + second.output_len(), b"one\ntwo\n".len());
    Ok(())
}
```

In a pipeline, consume each initialized output prefix before reusing the buffer.
See [installation and feature flags](#installation-and-feature-flags) for
allocation-free and alloc-only configurations.

### Custom normalizers and buffer safety

Custom normalizers remain supported. `NormalizeChunk` is an unsafe trait:
implement it with `unsafe impl` only after auditing every method against its
[documented safety contract](https://docs.rs/eolify/latest/eolify/trait.NormalizeChunk.html#safety).
Successful calls must report an initialized prefix within the output buffer;
never read uninitialized bytes or access buffers out of bounds. Insufficient
space must return an error, and `max_output_size_for_chunk` must provide enough
space for the corresponding call. Existing initialized bytes must stay
initialized even on error or panic. Whole-buffer normalization (no initial
state, final chunk) must preserve valid UTF-8 so that `Normalize::normalize_str`
can safely construct a `String`.

The public `helpers::vec_to_uninit_mut` and `helpers::slice_to_uninit_mut`
conversions require an unsafe block: keep existing bytes initialized for the
entire borrow, including unwinding. Passing the view directly to a conforming
normalizer satisfies that obligation. For a vector with existing contents,
`Vec::spare_capacity_mut` is a safe alternative when only spare capacity is needed;
it starts after the current length rather than at the beginning of the allocation.
Built-in `LF`/`CRLF` normalization and I/O extension methods remain safe to call.

For the 0.3 → 0.4 API changes, see the [migration notes](CHANGELOG.md#migration-notes).

## Installation and feature flags

```toml
[dependencies]
eolify = "0.5"
```

| Feature | Provides |
| --- | --- |
| `std` (default) | Synchronous I/O adapters, reader analysis, `alloc`, and `memchr` runtime CPU detection |
| `alloc` | Whole-buffer APIs returning `Vec` or `String` |
| `tokio` | Tokio async I/O adapters; enables `std` |
| `futures-io` | futures-io async I/O adapters; enables `std` |

Enable either or both async integrations as needed:

```toml
eolify = { version = "0.5", features = ["tokio", "futures-io"] }
```

For chunk normalization without `std` or a heap allocator:

```toml
eolify = { version = "0.5", default-features = false }
```

For in-memory APIs in a `no_std` application with an allocator:

```toml
eolify = { version = "0.5", default-features = false, features = ["alloc"] }
```

Requires Rust 1.81 or newer. See the [API documentation](https://docs.rs/eolify)
for additional methods and types.

## License

MIT or Apache-2.0, at your option.
