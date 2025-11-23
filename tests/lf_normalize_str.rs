use eolify::{Normalize, LF};

fn run(input: &str) -> String {
    LF::normalize_str(input)
}

#[test]
fn no_cr_or_lf() {
    let out = run("hello world");
    assert_eq!(out, "hello world".to_string());
}

#[test]
fn lone_cr_converted_to_lf() {
    let out = run("line1\rline2");
    assert_eq!(out, "line1\nline2".to_string());
}

#[test]
fn lone_lf_kept_as_lf() {
    let out = run("line1\nline2");
    assert_eq!(out, "line1\nline2".to_string());
}

#[test]
fn trailing_cr_emits_lf() {
    let out = run("foo\r");
    assert_eq!(out, "foo\n".to_string());
}

#[test]
fn _crlf_converted_to_lf() {
    let out = run("foo\r\nbar");
    assert_eq!(out, "foo\nbar".to_string());
}

#[test]
fn multiple_crs_and_crlf_mixed() {
    let out = run("\r\r\n");
    assert_eq!(out, "\n\n".to_string());
}
