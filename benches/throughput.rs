use core::fmt;
use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use eolify::{helpers::vec_to_uninit_mut, Normalize, NormalizeChunkResult, CRLF, LF};
use std::{mem::MaybeUninit, time::Duration};

/// Generate buffers with a few different patterns:
/// - "random": pseudo-random bytes (deterministic LCG)
/// - "all_lf": all newline bytes
/// - "all_cr": all CR bytes
/// - "crlf": repeating CRLF sequences
/// - "mixed": intermittent lone CRs and LFs
fn make_buffer(size: usize, pattern: &str) -> Vec<u8> {
    let mut v = vec![0u8; size];
    match pattern {
        "random" => {
            // simple LCG to avoid extra deps
            let mut state: u64 = 0x12345678;
            for i in 0..size {
                state = state.wrapping_mul(6364136223846793005).wrapping_add(1);
                v[i] = (state & 0xFF) as u8;
            }
        }
        "all_lf" => {
            for b in &mut v[..] {
                *b = b'\n';
            }
        }
        "all_cr" => {
            for b in &mut v[..] {
                *b = b'\r';
            }
        }
        "crlf" => {
            for i in 0..size {
                v[i] = if i % 2 == 0 { b'\r' } else { b'\n' };
            }
        }
        "mixed" => {
            for i in 0..size {
                v[i] = match i % 7 {
                    0 => b'\r',
                    1 => b'\n',
                    2 => b'a',
                    _ => b'b',
                }
            }
        }
        _ => {}
    }
    v
}

enum Format {
    CRLF,
    LF,
}

impl fmt::Display for Format {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Format::CRLF => write!(f, "crlf"),
            Format::LF => write!(f, "lf"),
        }
    }
}

impl Format {
    fn normalize_chunk(
        &self,
        input: &[u8],
        output: &mut [MaybeUninit<u8>],
        preceded_by_cr: bool,
        is_last_chunk: bool,
    ) -> eolify::Result<NormalizeChunkResult> {
        match self {
            Format::CRLF => CRLF::normalize_chunk(input, output, preceded_by_cr, is_last_chunk),
            Format::LF => LF::normalize_chunk(input, output, preceded_by_cr, is_last_chunk),
        }
    }
}

fn bench_throughput(c: &mut Criterion) {
    let formats = [Format::CRLF, Format::LF];

    for format in formats {
        let mut group1 = c.benchmark_group(format!("{format}_throughput"));
        // Longer measurement to get stable GB/s numbers
        group1.measurement_time(Duration::from_secs(3));
        group1.sample_size(10);

        let sizes = [8 << 10, 1 << 20]; // 8KiB, 1MiB
        let patterns = ["random", "all_lf", "all_cr", "crlf", "mixed"];

        for &size in &sizes {
            for &pattern in &patterns {
                let buf = make_buffer(size, pattern);
                let id = BenchmarkId::new(pattern, size);
                group1.throughput(Throughput::Bytes(size as u64));
                group1.bench_with_input(id, &buf, |b, data| {
                    // pre-allocate once (avoid measuring allocation)
                    let mut out = Vec::with_capacity(data.len() * 3 + 8);
                    b.iter(|| {
                        let status = format
                            .normalize_chunk(data, vec_to_uninit_mut(&mut out), false, false)
                            .unwrap();
                        std::hint::black_box(status.output_len());
                        std::hint::black_box(status.ended_with_cr());
                    })
                });
            }
        }

        group1.finish();

        // --- Chunked processing benchmark: process a large buffer in fixed-size chunks,
        // varying chunk size and preserving last_was_cr across chunk boundaries.
        let mut group2 = c.benchmark_group(format!("{format}_chunked"));
        group2.measurement_time(Duration::from_secs(3));
        group2.sample_size(10);

        let max_size = 64 << 20; // largest block to process (64MiB)
        let chunk_sizes = [1 << 10, 8 << 10, 16 << 10]; // 1K, 8K, 16K
                                                        // reuse the same patterns as above
        for &pattern in &patterns {
            let data = make_buffer(max_size, pattern);
            // precompute max chunk so we can allocate a single reusable output buffer
            let max_chunk = *chunk_sizes.iter().max().unwrap();
            // output buffer sized for the largest chunk (safe for any chunk size)
            let mut out = Vec::with_capacity(max_chunk * 3 + 8);

            for &chunk in &chunk_sizes {
                let id = BenchmarkId::new(format!("{pattern}/chunk-{chunk}"), max_size);
                group2.throughput(Throughput::Bytes(max_size as u64));
                group2.bench_with_input(id, &data, |b, input| {
                    // out is captured from outer scope and reused; avoid allocating inside iter.
                    b.iter(|| {
                        let mut last_was_cr = false;
                        // process the buffer in fixed-size chunks; pass last flag across chunks
                        for ch in input.chunks(chunk) {
                            let status = format
                                .normalize_chunk(
                                    ch,
                                    vec_to_uninit_mut(&mut out),
                                    last_was_cr,
                                    false,
                                )
                                .unwrap();
                            std::hint::black_box(status.output_len());
                            last_was_cr = status.ended_with_cr();
                        }
                        std::hint::black_box(last_was_cr);
                    })
                });
            }
        }

        group2.finish();
    }
}

criterion_group!(benches, bench_throughput);
criterion_main!(benches);
