#![cfg(feature = "futures-io")]

use std::{
    pin::{pin, Pin},
    task::{Context, Poll},
};

use eolify::async_io::crlf::NormalizingReader;
#[cfg(feature = "futures-io")]
use futures_util::AsyncReadExt as _;
#[cfg(feature = "tokio-io")]
use tokio::io::AsyncReadExt as _;

#[cfg(feature = "futures-io")]
#[async_std::test]
async fn crlf_split_across_readers() {
    let readers = vec![b"foo\r".as_ref(), b"\nbar".as_ref()].into_iter();
    let test_reader = AsyncTestReader::futures_new(readers);
    let nr = NormalizingReader::futures_with_size(test_reader, 3);
    let out = futures_read_all(nr).await;
    assert_eq!(out, b"foo\r\nbar".to_vec());
}

#[cfg(feature = "tokio-io")]
#[tokio::test]
async fn crlf_split_across_readers_tokio() {
    let readers = vec![b"foo\r".as_ref(), b"\nbar".as_ref()].into_iter();
    let test_reader = AsyncTestReader::tokio_new(readers);
    let nr = NormalizingReader::tokio_with_size(test_reader, 3);
    let out = tokio_read_all(nr).await;
    assert_eq!(out, b"foo\r\nbar".to_vec());
}

#[cfg(feature = "futures-io")]
#[async_std::test]
async fn crlf_split_across_three_readers() {
    let readers = vec![b"\r".as_ref(), b"".as_ref(), b"\n".as_ref()].into_iter();
    let test_reader = AsyncTestReader::futures_new(readers);
    let nr = NormalizingReader::futures_with_size(test_reader, 3);
    let out = futures_read_all(nr).await;
    assert_eq!(out, b"\r\n".to_vec());
}

#[cfg(feature = "futures-io")]
#[async_std::test]
async fn lone_lf_in_first_reader_converted_to_crlf() {
    let readers = vec![b"line1\n".as_ref(), b"line2".as_ref()].into_iter();
    let test_reader = AsyncTestReader::futures_new(readers);
    let nr = NormalizingReader::futures_with_size(test_reader, 4);
    let out = futures_read_all(nr).await;
    assert_eq!(out, b"line1\r\nline2".to_vec());
}

#[cfg(feature = "futures-io")]
#[async_std::test]
async fn multiple_crs_and_crlf_mixed_across_boundaries() {
    let readers = vec![b"\r".as_ref(), b"\r\n".as_ref()].into_iter();
    let test_reader = AsyncTestReader::futures_new(readers);
    let nr = NormalizingReader::futures_with_size(test_reader, 2);
    let out = futures_read_all(nr).await;
    assert_eq!(out, b"\r\n\r\n".to_vec());
}

#[cfg(feature = "futures-io")]
#[async_std::test]
async fn trailing_cr_at_eof_emits_crlf() {
    let readers = vec![b"foo\r".as_ref()].into_iter();
    let test_reader = AsyncTestReader::futures_new(readers);
    let nr = NormalizingReader::futures_with_size(test_reader, 4);
    let out = futures_read_all(nr).await;
    assert_eq!(out, b"foo\r\n".to_vec());
}

pub struct AsyncTestReader<R, I> {
    readers: I,
    current: Option<R>,
}

#[cfg(feature = "futures-io")]
impl<R: futures_io::AsyncRead, I: Iterator<Item = R>> AsyncTestReader<R, I> {
    pub fn futures_new(mut readers: I) -> AsyncTestReader<R, I> {
        let current = readers.next();
        AsyncTestReader {
            readers: readers,
            current: current,
        }
    }
}

#[cfg(feature = "tokio-io")]
impl<R: tokio::io::AsyncRead, I: Iterator<Item = R>> AsyncTestReader<R, I> {
    pub fn tokio_new(mut readers: I) -> AsyncTestReader<R, I> {
        let current = readers.next();
        AsyncTestReader {
            readers: readers,
            current: current,
        }
    }
}

#[cfg(feature = "futures-io")]
pub async fn futures_read_all<R: futures_io::AsyncRead + Unpin>(mut r: R) -> Vec<u8> {
    let mut out = Vec::new();
    r.read_to_end(&mut out).await.unwrap();
    out
}

#[cfg(feature = "tokio-io")]
pub async fn tokio_read_all<R: tokio::io::AsyncRead + Unpin>(mut r: R) -> Vec<u8> {
    let mut out = Vec::new();
    r.read_to_end(&mut out).await.unwrap();
    out
}

#[cfg(feature = "futures-io")]
impl<R: futures_io::AsyncRead + Unpin, I: Iterator<Item = R> + Unpin> futures_io::AsyncRead
    for AsyncTestReader<R, I>
{
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut [u8],
    ) -> Poll<std::io::Result<usize>> {
        let this = self.get_mut();
        loop {
            match this.current.as_mut() {
                Some(r) => {
                    let mut r = pin!(r);
                    match r.as_mut().poll_read(cx, buf) {
                        Poll::Ready(Ok(n)) => {
                            if n > 0 {
                                return Poll::Ready(Ok(n));
                            }
                        }
                        Poll::Ready(Err(e)) => return Poll::Ready(Err(e)),
                        Poll::Pending => return Poll::Pending,
                    }
                }
                None => return Poll::Ready(Ok(0)),
            }
            this.current = this.readers.next();
        }
    }
}
#[cfg(feature = "tokio-io")]
impl<R: tokio::io::AsyncRead + Unpin, I: Iterator<Item = R> + Unpin> tokio::io::AsyncRead
    for AsyncTestReader<R, I>
{
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut tokio::io::ReadBuf<'_>,
    ) -> Poll<std::io::Result<()>> {
        let this = self.get_mut();
        loop {
            match this.current.as_mut() {
                Some(r) => {
                    let mut r = pin!(r);
                    let len = buf.filled().len();
                    match r.as_mut().poll_read(cx, buf) {
                        Poll::Ready(Ok(())) => {
                            if buf.filled().len() > len {
                                return Poll::Ready(Ok(()));
                            }
                        }
                        Poll::Ready(Err(e)) => return Poll::Ready(Err(e)),
                        Poll::Pending => return Poll::Pending,
                    }
                }
                None => return Poll::Ready(Ok(())),
            }
            this.current = this.readers.next();
        }
    }
}
