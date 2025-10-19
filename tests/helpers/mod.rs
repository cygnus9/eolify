use std::io::{self, Read};

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
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
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
