use eolify::{Normalize, CRLF};

fn run(input: &[u8]) -> Vec<u8> {
    CRLF::normalize(input)
}

#[test]
fn no_cr_or_lf() {
    let out = run(b"hello world");
    assert_eq!(out, b"hello world".to_vec());
}

#[test]
fn lone_lf_converted_to_crlf() {
    let out = run(b"line1\nline2");
    assert_eq!(out, b"line1\r\nline2".to_vec());
}

#[test]
fn trailing_cr_emits_crlf() {
    let out = run(b"foo\r");
    assert_eq!(out, b"foo\r\n".to_vec());
}

#[test]
fn cr_not_followed_by_lf_in_middle_becomes_crlf() {
    let out = run(b"a\rb");
    assert_eq!(out, b"a\r\nb".to_vec());
}

#[test]
fn existing_crlf_kept_as_crlf() {
    let out = run(b"foo\r\nbar");
    assert_eq!(out, b"foo\r\nbar".to_vec());
}

#[test]
fn multiple_crs_and_crlf_mixed() {
    let out = run(b"\r\r\n");
    assert_eq!(out, b"\r\n\r\n".to_vec());
}
