use std::io::Write;

use eolify::{IoExt, WriteExt, CRLF};

#[test]
fn crlf_split_across_chunks() {
    let mut writer = CRLF::wrap_writer_with_buffer_size(Vec::new(), 4);
    writer.write_all(b"foo\r").unwrap();
    writer.write_all(b"\nbar").unwrap();
    let out = writer.finish().unwrap();
    assert_eq!(out, b"foo\r\nbar".to_vec());
}

#[test]
fn crlf_split_across_three_chunks() {
    let mut writer = CRLF::wrap_writer_with_buffer_size(Vec::new(), 4);
    writer.write_all(b"foo\r").unwrap();
    writer.flush().unwrap();
    writer.write_all(b"\nbar").unwrap();
    let out = writer.finish().unwrap();
    assert_eq!(out, b"foo\r\nbar".to_vec());
}

#[test]
fn lone_lf_in_first_chunk_converted_to_crlf() {
    let mut writer = CRLF::wrap_writer_with_buffer_size(Vec::new(), 5);
    writer.write_all(b"line1\n").unwrap();
    writer.write_all(b"line2").unwrap();
    let out = writer.finish().unwrap();
    assert_eq!(out, b"line1\r\nline2".to_vec());
}

#[test]
fn multiple_crs_and_crlf_mixed_across_boundaries() {
    let mut writer = CRLF::wrap_writer_with_buffer_size(Vec::new(), 1);
    writer.write_all(b"\r").unwrap();
    writer.write_all(b"\r\n").unwrap();
    let out = writer.finish().unwrap();
    assert_eq!(out, b"\r\n\r\n".to_vec());
}

#[test]
fn trailing_cr_at_eof_emits_crlf() {
    let mut writer = CRLF::wrap_writer_with_buffer_size(Vec::new(), 16);
    writer.write_all(b"foo\r").unwrap();
    let out = writer.finish().unwrap();
    assert_eq!(out, b"foo\r\n".to_vec());
}

#[test]
fn extension_trait() {
    let mut writer = Vec::new().normalize_newlines(CRLF);
    writer.write_all(b"\r").unwrap();
    writer.write_all(b"\r\n").unwrap();
    let out = writer.finish().unwrap();
    assert_eq!(out, b"\r\n\r\n".to_vec());
}
