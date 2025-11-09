use eolify::core::crlf::normalize_str;

fn run(input: &str) -> String {
    normalize_str(input)
}

#[test]
fn no_cr_or_lf() {
    let out = run("hello world");
    assert_eq!(out, "hello world".to_string());
}

#[test]
fn lone_lf_converted_to_crlf() {
    let out = run("line1\nline2");
    assert_eq!(out, "line1\r\nline2".to_string());
}

#[test]
fn trailing_cr_emits_crlf() {
    let out = run("foo\r");
    assert_eq!(out, "foo\r\n".to_string());
}

#[test]
fn cr_not_followed_by_lf_in_middle_becomes_crlf() {
    let out = run("a\r");
    assert_eq!(out, "a\r\n".to_string());
}

#[test]
fn existing_crlf_kept_as_crlf() {
    let out = run("foo\r\nbar");
    assert_eq!(out, "foo\r\nbar".to_string());
}

#[test]
fn multiple_crs_and_crlf_mixed() {
    let out = run("\r\r\n");
    assert_eq!(out, "\r\n\r\n".to_string());
}
