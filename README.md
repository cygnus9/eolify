# eolify

High-performance line ending normalization for Rust.

eolify is a lightweight, allocation-conscious library for normalizing end-of-line (EOL) sequences in large text streams
or buffers. It’s designed for high-throughput processing pipelines, data ingestion systems, and cross-platform tooling
where consistency and efficiency matter.

## Features

* Fast and memory-efficient — optimized for bulk text processing
* Normalizes EOLs to a consistent format (`\r\n` for now)
* Minimal dependencies — ideal for embedding in performance-critical code
* Handles mixed endings (`\n`, `\r\n`, `\r`) gracefully
* Built with large-scale text data and streaming I/O in mind

## Current status

Currently supports: normalization to CRLF (`\r\n`) using a chunk based API or through a `Read` or `Write` implementation.

### Planned:

* LF (`\n`) normalization
* `AsyncRead` / `AsyncWrite`

## Example
```rust
use eolify::core::crlf;

let text = "one\nline\r\ntwo\rthree";
let normalized = crlf::normalize_str(text);
assert_eq!(normalized, "one\r\nline\r\ntwo\r\nthree");
```

## License

MIT or Apache-2.0, at your option.