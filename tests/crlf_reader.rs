use std::io::Read;

use eolify::io::crlf::NormalizingReader;

#[test]
fn crlf_split_across_readers() {
    let readers = vec![b"foo\r".as_ref(), b"\nbar".as_ref()].into_iter();
    let test_reader = TestReader::new(readers);
    let nr = NormalizingReader::with_size(test_reader, 3);
    let out = read_all(nr);
    assert_eq!(out, b"foo\r\nbar".to_vec());
}

#[test]
fn crlf_split_across_three_readers() {
    let readers = vec![b"\r".as_ref(), b"".as_ref(), b"\n".as_ref()].into_iter();
    let test_reader = TestReader::new(readers);
    let nr = NormalizingReader::with_size(test_reader, 3);
    let out = read_all(nr);
    assert_eq!(out, b"\r\n".to_vec());
}

#[test]
fn lone_lf_in_first_reader_converted_to_crlf() {
    let readers = vec![b"line1\n".as_ref(), b"line2".as_ref()].into_iter();
    let test_reader = TestReader::new(readers);
    let nr = NormalizingReader::with_size(test_reader, 4);
    let out = read_all(nr);
    assert_eq!(out, b"line1\r\nline2".to_vec());
}

#[test]
fn multiple_crs_and_crlf_mixed_across_boundaries() {
    let readers = vec![b"\r".as_ref(), b"\r\n".as_ref()].into_iter();
    let test_reader = TestReader::new(readers);
    let nr = NormalizingReader::with_size(test_reader, 2);
    let out = read_all(nr);
    assert_eq!(out, b"\r\n\r\n".to_vec());
}

#[test]
fn trailing_cr_at_eof_emits_crlf() {
    let readers = vec![b"foo\r".as_ref()].into_iter();
    let test_reader = TestReader::new(readers);
    let nr = NormalizingReader::with_size(test_reader, 4);
    let out = read_all(nr);
    assert_eq!(out, b"foo\r\n".to_vec());
}

fn read_all<R: Read>(mut r: R) -> Vec<u8> {
    let mut out = Vec::new();
    r.read_to_end(&mut out).unwrap();
    out
}

pub struct TestReader<R, I> {
    readers: I,
    current: Option<R>,
}

impl<R: Read, I: Iterator<Item = R>> TestReader<R, I> {
    pub fn new(mut readers: I) -> TestReader<R, I> {
        let current = readers.next();
        TestReader {
            readers: readers,
            current: current,
        }
    }
}

impl<R: Read, I: Iterator<Item = R>> Read for TestReader<R, I> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        loop {
            match self.current {
                Some(ref mut r) => {
                    let n = r.read(buf)?;
                    if n > 0 {
                        return Ok(n);
                    }
                }
                None => return Ok(0),
            }
            self.current = self.readers.next();
        }
    }
}
