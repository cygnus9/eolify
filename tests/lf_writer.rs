use std::io::Write;

use eolify::{IoExt, WriteExt, LF};

#[test]
fn crlf_split_across_chunks() {
    let mut writer = LF::wrap_writer_with_buffer_size(Vec::new(), 4);
    writer.write_all(b"foo\r").unwrap();
    writer.write_all(b"\nbar").unwrap();
    let out = writer.finish().unwrap();
    assert_eq!(out, b"foo\nbar".to_vec());
}

#[test]
fn crlf_split_across_three_chunks() {
    let mut writer = LF::wrap_writer_with_buffer_size(Vec::new(), 4);
    writer.write_all(b"foo\r").unwrap();
    writer.flush().unwrap();
    writer.write_all(b"\nbar").unwrap();
    let out = writer.finish().unwrap();
    assert_eq!(out, b"foo\nbar".to_vec());
}

#[test]
fn lone_lf_in_first_chunk_kept_as_lf() {
    let mut writer = LF::wrap_writer_with_buffer_size(Vec::new(), 5);
    writer.write_all(b"line1\n").unwrap();
    writer.write_all(b"line2").unwrap();
    let out = writer.finish().unwrap();
    assert_eq!(out, b"line1\nline2".to_vec());
}

#[test]
fn multiple_crs_and_crlf_mixed_across_boundaries() {
    let mut writer = LF::wrap_writer_with_buffer_size(Vec::new(), 1);
    writer.write_all(b"\r").unwrap();
    writer.write_all(b"\r\n").unwrap();
    let out = writer.finish().unwrap();
    assert_eq!(out, b"\n\n".to_vec());
}

#[test]
fn trailing_cr_at_eof_emits_lf() {
    let mut writer = LF::wrap_writer_with_buffer_size(Vec::new(), 16);
    writer.write_all(b"foo\r").unwrap();
    let out = writer.finish().unwrap();
    assert_eq!(out, b"foo\n".to_vec());
}

#[test]
fn extension_trait() {
    let mut writer = Vec::new().normalize_newlines(LF);
    writer.write_all(b"\r").unwrap();
    writer.write_all(b"\r\n").unwrap();
    let out = writer.finish().unwrap();
    assert_eq!(out, b"\n\n".to_vec());
}
