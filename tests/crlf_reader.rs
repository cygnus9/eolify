mod helpers;

use std::io::{Cursor, Read};

use eolify::io::crlf::NormalizingReader;
use helpers::TestReader;

fn read_all<R: Read>(mut r: R) -> Vec<u8> {
    let mut out = Vec::new();
    r.read_to_end(&mut out).unwrap();
    out
}

#[test]
fn crlf_split_across_readers() {
    let readers = vec![
        Cursor::new(b"foo\r".to_vec()),
        Cursor::new(b"\nbar".to_vec()),
    ]
    .into_iter();
    let test_reader = TestReader::new(readers);
    let nr = NormalizingReader::with_size(test_reader, 3);
    let out = read_all(nr);
    assert_eq!(out, b"foo\r\nbar".to_vec());
}

#[test]
fn crlf_split_across_three_readers() {
    let readers = vec![
        Cursor::new(b"\r".to_vec()),
        Cursor::new(b"".to_vec()),
        Cursor::new(b"\n".to_vec()),
    ]
    .into_iter();
    let test_reader = TestReader::new(readers);
    let nr = NormalizingReader::with_size(test_reader, 3);
    let out = read_all(nr);
    assert_eq!(out, b"\r\n".to_vec());
}

#[test]
fn lone_lf_in_first_reader_converted_to_crlf() {
    let readers = vec![
        Cursor::new(b"line1\n".to_vec()),
        Cursor::new(b"line2".to_vec()),
    ]
    .into_iter();
    let test_reader = TestReader::new(readers);
    let nr = NormalizingReader::with_size(test_reader, 4);
    let out = read_all(nr);
    assert_eq!(out, b"line1\r\nline2".to_vec());
}

#[test]
fn multiple_crs_and_crlf_mixed_across_boundaries() {
    let readers = vec![Cursor::new(b"\r".to_vec()), Cursor::new(b"\r\n".to_vec())].into_iter();
    let test_reader = TestReader::new(readers);
    let nr = NormalizingReader::with_size(test_reader, 2);
    let out = read_all(nr);
    assert_eq!(out, b"\r\n\r\n".to_vec());
}

#[test]
fn trailing_cr_at_eof_emits_crlf() {
    let readers = vec![Cursor::new(b"foo\r".to_vec())].into_iter();
    let test_reader = TestReader::new(readers);
    let nr = NormalizingReader::with_size(test_reader, 4);
    let out = read_all(nr);
    assert_eq!(out, b"foo\r\n".to_vec());
}
