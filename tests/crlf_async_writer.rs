#![cfg(any(feature = "futures-io", feature = "tokio"))]

use std::{
    pin::Pin,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc, Mutex,
    },
    task::{Context, Poll},
};

struct ZeroWriter;

struct FlushCounterWriter(Arc<AtomicUsize>);

struct PendingAfterPartialWrite {
    out: Arc<Mutex<Vec<u8>>>,
    pending_after_writes: usize,
    write_calls: usize,
    max_ready_bytes: usize,
}

impl PendingAfterPartialWrite {
    fn new(out: Arc<Mutex<Vec<u8>>>) -> Self {
        Self {
            out,
            pending_after_writes: 1,
            write_calls: 0,
            max_ready_bytes: 2,
        }
    }

    fn poll_write_inner(
        &mut self,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<std::io::Result<usize>> {
        if self.write_calls == self.pending_after_writes {
            self.write_calls += 1;
            cx.waker().wake_by_ref();
            return Poll::Pending;
        }

        self.write_calls += 1;
        let bytes_now = buf.len().min(self.max_ready_bytes);
        self.out
            .lock()
            .unwrap()
            .extend_from_slice(&buf[..bytes_now]);
        Poll::Ready(Ok(bytes_now))
    }
}

#[cfg(feature = "futures-io")]
impl futures_io::AsyncWrite for ZeroWriter {
    fn poll_write(
        self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        _buf: &[u8],
    ) -> Poll<std::io::Result<usize>> {
        Poll::Ready(Ok(0))
    }

    fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        Poll::Ready(Ok(()))
    }

    fn poll_close(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        Poll::Ready(Ok(()))
    }
}

#[cfg(feature = "futures-io")]
impl futures_io::AsyncWrite for FlushCounterWriter {
    fn poll_write(
        self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<std::io::Result<usize>> {
        Poll::Ready(Ok(buf.len()))
    }

    fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        self.0.fetch_add(1, Ordering::SeqCst);
        Poll::Ready(Ok(()))
    }

    fn poll_close(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        Poll::Ready(Ok(()))
    }
}

#[cfg(feature = "futures-io")]
impl futures_io::AsyncWrite for PendingAfterPartialWrite {
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<std::io::Result<usize>> {
        self.as_mut().get_mut().poll_write_inner(cx, buf)
    }

    fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        Poll::Ready(Ok(()))
    }

    fn poll_close(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        Poll::Ready(Ok(()))
    }
}

#[cfg(feature = "tokio")]
impl tokio::io::AsyncWrite for ZeroWriter {
    fn poll_write(
        self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        _buf: &[u8],
    ) -> Poll<std::io::Result<usize>> {
        Poll::Ready(Ok(0))
    }

    fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        Poll::Ready(Ok(()))
    }

    fn poll_shutdown(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        Poll::Ready(Ok(()))
    }
}

#[cfg(feature = "tokio")]
impl tokio::io::AsyncWrite for FlushCounterWriter {
    fn poll_write(
        self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<std::io::Result<usize>> {
        Poll::Ready(Ok(buf.len()))
    }

    fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        self.0.fetch_add(1, Ordering::SeqCst);
        Poll::Ready(Ok(()))
    }

    fn poll_shutdown(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        Poll::Ready(Ok(()))
    }
}

#[cfg(feature = "tokio")]
impl tokio::io::AsyncWrite for PendingAfterPartialWrite {
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<std::io::Result<usize>> {
        self.as_mut().get_mut().poll_write_inner(cx, buf)
    }

    fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        Poll::Ready(Ok(()))
    }

    fn poll_shutdown(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        Poll::Ready(Ok(()))
    }
}

macro_rules! dual_test {
    ($name:ident, $body:block) => {
        mod $name {
            use eolify::CRLF;

            #[cfg(feature = "futures-io")]
            #[async_std::test]
            async fn futures_io() {
                use eolify::FuturesIoExt;
                use futures_util::AsyncWriteExt;

                $body
            }

            #[cfg(feature = "tokio")]
            #[tokio::test]
            async fn tokio() {
                use eolify::TokioExt;
                use tokio::io::AsyncWriteExt;

                $body
            }
        }
    };
}

dual_test!(crlf_split_across_chunks, {
    let mut writer = CRLF::wrap_async_writer_with_buffer_size(Vec::new(), 4);
    writer.write_all(b"foo\r").await.unwrap();
    writer.write_all(b"\nbar").await.unwrap();
    let out = writer.finish().await.unwrap();
    assert_eq!(out, b"foo\r\nbar".to_vec());
});

dual_test!(crlf_split_across_three_chunks, {
    let mut writer = CRLF::wrap_async_writer_with_buffer_size(Vec::new(), 4);
    writer.write_all(b"foo\r").await.unwrap();
    writer.flush().await.unwrap();
    writer.write_all(b"\nbar").await.unwrap();
    let out = writer.finish().await.unwrap();
    assert_eq!(out, b"foo\r\nbar".to_vec());
});

dual_test!(lone_lf_in_first_chunk_converted_to_crlf, {
    let mut writer = CRLF::wrap_async_writer_with_buffer_size(Vec::new(), 5);
    writer.write_all(b"line1\n").await.unwrap();
    writer.write_all(b"line2").await.unwrap();
    let out = writer.finish().await.unwrap();
    assert_eq!(out, b"line1\r\nline2".to_vec());
});

dual_test!(multiple_crs_and_crlf_mixed_across_boundaries, {
    let mut writer = CRLF::wrap_async_writer_with_buffer_size(Vec::new(), 1);
    writer.write_all(b"\r").await.unwrap();
    writer.write_all(b"\r\n").await.unwrap();
    let out = writer.finish().await.unwrap();
    assert_eq!(out, b"\r\n\r\n".to_vec());
});

dual_test!(trailing_cr_at_eof_emits_crlf, {
    let mut writer = CRLF::wrap_async_writer_with_buffer_size(Vec::new(), 16);
    writer.write_all(b"foo\r").await.unwrap();
    let out = writer.finish().await.unwrap();
    assert_eq!(out, b"foo\r\n".to_vec());
});

dual_test!(inner_zero_write_returns_write_zero, {
    let mut writer = CRLF::wrap_async_writer_with_buffer_size(super::ZeroWriter, 1);
    let err = writer.write_all(b"\n").await.unwrap_err();
    assert_eq!(err.kind(), std::io::ErrorKind::WriteZero);
});

dual_test!(flush_reaches_inner_writer_without_pending_output, {
    let flushes = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let mut writer =
        CRLF::wrap_async_writer_with_buffer_size(super::FlushCounterWriter(flushes.clone()), 4);

    writer.flush().await.unwrap();

    assert_eq!(flushes.load(std::sync::atomic::Ordering::SeqCst), 1);
});

dual_test!(pending_after_buffered_input_does_not_duplicate_output, {
    let out = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let inner = super::PendingAfterPartialWrite::new(out.clone());
    let mut writer = CRLF::wrap_async_writer_with_buffer_size(inner, 4);

    writer.write_all(b"a\nbc").await.unwrap();
    writer.finish().await.unwrap();

    assert_eq!(*out.lock().unwrap(), b"a\r\nbc".to_vec());
});
