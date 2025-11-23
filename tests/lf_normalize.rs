use eolify::{Normalize, LF};

fn run(input: &[u8]) -> Vec<u8> {
    LF::normalize(input)
}

#[test]
fn no_cr_or_lf() {
    let out = run(b"hello world");
    assert_eq!(out, b"hello world".to_vec());
}

#[test]
fn lone_cr_converted_to_lf() {
    let out = run(b"line1\rline2");
    assert_eq!(out, b"line1\nline2".to_vec());
}

#[test]
fn lone_lf_kept_as_lf() {
    let out = run(b"line1\nline2");
    assert_eq!(out, b"line1\nline2".to_vec());
}

#[test]
fn trailing_cr_emits_lf() {
    let out = run(b"foo\r");
    assert_eq!(out, b"foo\n".to_vec());
}

#[test]
fn _crlf_converted_to_lf() {
    let out = run(b"foo\r\nbar");
    assert_eq!(out, b"foo\nbar".to_vec());
}

#[test]
fn multiple_crs_and_crlf_mixed() {
    let out = run(b"\r\r\n");
    assert_eq!(out, b"\n\n".to_vec());
}
