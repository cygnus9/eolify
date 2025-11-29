use eolify::{helpers::slice_to_uninit_mut, NormalizeChunk, LF};

fn run(input: &[u8], preceded_by_cr: bool, is_last_chunk: bool) -> (Vec<u8>, bool) {
    let mut output = [0; 32];
    let status = LF::normalize_chunk(
        input,
        slice_to_uninit_mut(&mut output),
        preceded_by_cr,
        is_last_chunk,
    )
    .unwrap();
    (
        output[..status.output_len()].to_vec(),
        status.ended_with_cr(),
    )
}

#[test]
fn no_cr_or_lf() {
    let (out, last) = run(b"hello world", false, false);
    assert_eq!(out, b"hello world");
    assert_eq!(last, false);
}

#[test]
fn lone_lf_kept_as_lf() {
    let (out, last) = run(b"line1\nline2", false, false);
    assert_eq!(out, b"line1\nline2");
    assert_eq!(last, false);
}

#[test]
fn trailing_cr_emits_lf_and_sets_last_flag() {
    let (out, last) = run(b"foo\r", false, false);
    assert_eq!(out, b"foo\n");
    assert_eq!(last, true);
}

#[test]
fn trailing_cr_in_last_chunk_emits_lf() {
    let (out, last) = run(b"foo\r", false, true);
    assert_eq!(out, b"foo\n");
    assert_eq!(last, false);
}

#[test]
fn cr_not_followed_by_lf_in_middle_becomes_lf() {
    let (out, last) = run(b"a\rb", false, false);
    assert_eq!(out, b"a\nb");
    assert_eq!(last, false);
}

#[test]
fn crlf_converted_to_lf() {
    let (out, last) = run(b"foo\r\nbar", false, false);
    assert_eq!(out, b"foo\nbar");
    assert_eq!(last, false);
}

#[test]
fn last_was_cr_and_next_is_lf_skips_lf() {
    let (out, last) = run(b"\nabc", true, false);
    assert_eq!(out, b"abc");
    assert_eq!(last, false);
}

#[test]
fn last_was_cr_and_next_is_not_lf_skips_nothing() {
    let (out, last) = run(b"X", true, false);
    assert_eq!(out, b"X");
    assert_eq!(last, false);
}

#[test]
fn last_was_cr_and_empty_chunk_does_not_emit_lf_if_not_last() {
    let (out, last) = run(b"", true, false);
    assert_eq!(out, b"");
    assert_eq!(last, true);
}

#[test]
fn last_was_cr_and_empty_chunk_does_not_emit_lf_if_last() {
    let (out, last) = run(b"", true, true);
    assert_eq!(out, b"");
    assert_eq!(last, false);
}

#[test]
fn multiple_crs_and_crlf_mixed() {
    let (out, last) = run(b"\r\r\n", false, false);
    assert_eq!(out, b"\n\n");
    assert_eq!(last, false);
}
