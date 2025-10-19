use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use eolify::crlf;
use std::time::Duration;

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

fn bench_throughput(c: &mut Criterion) {
    let mut group = c.benchmark_group("crlf_throughput");
    // Longer measurement to get stable GB/s numbers
    group.measurement_time(Duration::from_secs(10));
    group.sample_size(10);

    let sizes = [1 << 20, 8 << 20, 64 << 20]; // 1MiB, 8MiB, 64MiB
    let patterns = ["random", "all_lf", "all_cr", "crlf", "mixed"];

    for &size in &sizes {
        for &pattern in &patterns {
            let buf = make_buffer(size, pattern);
            let id = BenchmarkId::new(pattern, size);
            group.throughput(Throughput::Bytes(size as u64));
            group.bench_with_input(id, &buf, |b, data| {
                // pre-allocate once (avoid measuring allocation)
                let mut out = vec![0u8; data.len() * 3 + 8];
                b.iter(|| {
                    let status = crlf::normalize_chunk(data, &mut out, false, false).unwrap();
                    std::hint::black_box(status.output_len());
                    std::hint::black_box(status.ended_with_cr());
                })
            });
        }
    }

    group.finish();

    // --- Chunked processing benchmark: process a large buffer in fixed-size chunks,
    // varying chunk size and preserving last_was_cr across chunk boundaries.
    let mut group2 = c.benchmark_group("crlf_chunked");
    group2.measurement_time(Duration::from_secs(10));
    group2.sample_size(10);

    let max_size = 64 << 20; // largest block to process (64MiB)
    let chunk_sizes = [4 << 10, 8 << 10, 16 << 10, 32 << 10, 64 << 10, 128 << 10]; // 4K..128K
                                                                                   // reuse the same patterns as above
    for &pattern in &patterns {
        let data = make_buffer(max_size, pattern);
        // precompute max chunk so we can allocate a single reusable output buffer
        let max_chunk = *chunk_sizes.iter().max().unwrap();
        // output buffer sized for the largest chunk (safe for any chunk size)
        let mut out = vec![0u8; max_chunk * 3 + 8];

        for &chunk in &chunk_sizes {
            let id = BenchmarkId::new(format!("{pattern}/chunk-{chunk}"), max_size);
            group2.throughput(Throughput::Bytes(max_size as u64));
            group2.bench_with_input(id, &data, |b, input| {
                // out is captured from outer scope and reused; avoid allocating inside iter.
                b.iter(|| {
                    let mut last_was_cr = false;
                    // process the buffer in fixed-size chunks; pass last flag across chunks
                    for ch in input.chunks(chunk) {
                        let status =
                            crlf::normalize_chunk(ch, &mut out, last_was_cr, false).unwrap();
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

criterion_group!(benches, bench_throughput);
criterion_main!(benches);
