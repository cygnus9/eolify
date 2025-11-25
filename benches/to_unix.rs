use std::{hint::black_box, sync::LazyLock};

use criterion::{criterion_group, criterion_main, Criterion};
use eolify::{Normalize, LF};
use newline_normalizer::{ToDosNewlines, ToUnixNewlines};
use regex::Regex;

static UNIX_REGEX: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\r\n?").unwrap());

fn bench_to_unix_newlines(c: &mut Criterion) {
    let input = "
      Это пример параграфа с пробелами и юникодом.\r\n
    Он содержит строки на русском языке, немного английского, и даже: こんにちは世界！\r

    Here's a sentence with normal ASCII characters, leading spaces, and symbols: @$%&.\r

            مرحبا بك في عالم الترميز الموحد.
    "
    .to_string();
    let pre_normalized_input = LF::normalize_str(&input);
    assert_eq!(pre_normalized_input, UNIX_REGEX.replace_all(&input, "\n"));
    assert_eq!(pre_normalized_input, newline_converter::dos2unix(&input));
    assert_eq!(pre_normalized_input, input.to_unix_newlines());

    let large_input = include_str!("./files/sherlock.txt")
        .to_dos_newlines()
        .to_string(); // input text has all new lines in Unix format, so we have to convert it to DOS format first for the sake of the benchmark
    let pre_normalized_large_input = LF::normalize_str(&large_input);
    assert_eq!(
        pre_normalized_large_input,
        UNIX_REGEX.replace_all(&large_input, "\n")
    );
    assert_eq!(
        pre_normalized_large_input,
        newline_converter::dos2unix(&large_input)
    );
    assert_eq!(pre_normalized_large_input, large_input.to_unix_newlines());

    c.bench_function("regex", |b| {
        b.iter(|| UNIX_REGEX.replace_all(black_box(&input), "\n"))
    });

    c.bench_function("regex with pre-normalized text", |b| {
        b.iter(|| UNIX_REGEX.replace_all(black_box(&pre_normalized_input), "\n"))
    });

    c.bench_function("regex with large ASCII text", |b| {
        b.iter(|| UNIX_REGEX.replace_all(black_box(&large_input), "\n"))
    });

    c.bench_function("regex with large pre-normalized ASCII text", |b| {
        b.iter(|| UNIX_REGEX.replace_all(black_box(&pre_normalized_large_input), "\n"))
    });

    c.bench_function("3rd party crate \"newline-converter\": dos2unix()", |b| {
        b.iter(|| newline_converter::dos2unix(black_box(&input)))
    });

    c.bench_function(
        "3rd party crate \"newline-converter\": dos2unix() with pre-normalized text",
        |b| b.iter(|| newline_converter::dos2unix(black_box(&pre_normalized_input))),
    );

    c.bench_function(
        "3rd party crate \"newline-converter\": dos2unix() with large ASCII text",
        |b| b.iter(|| newline_converter::dos2unix(black_box(&large_input))),
    );

    c.bench_function(
        "3rd party crate \"newline-converter\": dos2unix() with large pre-normalized ASCII text",
        |b| b.iter(|| newline_converter::dos2unix(black_box(&pre_normalized_large_input))),
    );

    c.bench_function(
        "3rd party crate \"newline_normalizer\": to_unix_newlines()",
        |b| {
            let input_slice = input.as_str();
            b.iter(|| newline_normalizer::ToUnixNewlines::to_unix_newlines(black_box(input_slice)))
        },
    );

    c.bench_function(
        "3rd party crate \"newline_normalizer\": to_unix_newlines() with pre-normalized text",
        |b| {
            let input_slice = pre_normalized_input.as_str();
            b.iter(|| newline_normalizer::ToUnixNewlines::to_unix_newlines(black_box(input_slice)))
        },
    );

    c.bench_function(
        "3rd party crate \"newline_normalizer\": to_unix_newlines() with large ASCII text",
        |b| {
            let input_slice = large_input.as_str();
            b.iter(|| newline_normalizer::ToUnixNewlines::to_unix_newlines(black_box(input_slice)))
        },
    );

    c.bench_function("3rd party crate \"newline_normalizer\": to_unix_newlines() with pre-normalized large ASCII text", |b| {
        let input_slice = pre_normalized_large_input.as_str();
        b.iter(|| newline_normalizer::ToUnixNewlines::to_unix_newlines(black_box(input_slice)))
    });

    c.bench_function("this crate: LF::normalize_str()", |b| {
        let input_slice = input.as_str();
        b.iter(|| LF::normalize_str(black_box(input_slice)))
    });

    c.bench_function(
        "this crate: LF::normalize_str() with pre-normalized text",
        |b| {
            let input_slice = pre_normalized_input.as_str();
            b.iter(|| LF::normalize_str(black_box(input_slice)))
        },
    );

    c.bench_function(
        "this crate: LF::normalize_str() with large ASCII text",
        |b| {
            let input_slice = large_input.as_str();
            b.iter(|| LF::normalize_str(black_box(input_slice)))
        },
    );

    c.bench_function(
        "this crate: LF::normalize_str() with pre-normalized large ASCII text",
        |b| {
            let input_slice = pre_normalized_large_input.as_str();
            b.iter(|| LF::normalize_str(black_box(input_slice)))
        },
    );
}

criterion_group!(benches, bench_to_unix_newlines);
criterion_main!(benches);
