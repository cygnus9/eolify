# eolify  
**High-performance line ending normalization for Rust**  

`eolify` is a lightweight, allocation-conscious library for normalizing end-of-line (EOL) sequences in large text streams or buffers. It’s designed for high-throughput processing pipelines, data ingestion systems, and cross-platform tooling where consistency and efficiency matter.  

## Features  
- Fast and memory-efficient — optimized for bulk text processing.  
- Normalizes EOLs to a consistent format (currently CRLF `\r\n`).
- Minimal dependencies — ideal for embedding in performance-critical code.  
- Handles mixed endings (`\n`, `\r\n`, `\r`) gracefully.  
- Supports:  
  - Chunk-based API (buffer slices)  
  - Synchronous implementations of `Read` / `Write`  
  - Asynchronous implementations of `AsyncRead` / `AsyncWrite` (both `futures_io` and `tokio` supported).

## Current status  
- Normalization to CRLF (`\r\n`) is implemented.
- LF-only (`\n`) normalization is **planned** for a future version.

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
```rust
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
eolify = "0.3"
```

Then either call the high-level string routines (for small chunks) or use the I/O wrappers for streaming use-cases.

## License

MIT or Apache-2.0, at your option.
