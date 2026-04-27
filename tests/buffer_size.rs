use eolify::{IoExt, CRLF};

#[test]
#[should_panic(expected = "buffer size must be greater than zero")]
fn sync_reader_rejects_zero_buffer_size() {
    let _ = CRLF::wrap_reader_with_buffer_size(std::io::empty(), 0);
}

#[test]
#[should_panic(expected = "buffer size must be greater than zero")]
fn sync_writer_rejects_zero_buffer_size() {
    let _ = CRLF::wrap_writer_with_buffer_size(Vec::new(), 0);
}

#[cfg(feature = "futures-io")]
#[test]
#[should_panic(expected = "buffer size must be greater than zero")]
fn futures_reader_rejects_zero_buffer_size() {
    use eolify::FuturesIoExt;

    let _ = CRLF::wrap_async_reader_with_buffer_size(futures_util::io::empty(), 0);
}

#[cfg(feature = "futures-io")]
#[test]
#[should_panic(expected = "buffer size must be greater than zero")]
fn futures_writer_rejects_zero_buffer_size() {
    use eolify::FuturesIoExt;

    let _ = CRLF::wrap_async_writer_with_buffer_size(futures_util::io::sink(), 0);
}

#[cfg(feature = "tokio")]
#[test]
#[should_panic(expected = "buffer size must be greater than zero")]
fn tokio_reader_rejects_zero_buffer_size() {
    use eolify::TokioExt;

    let _ = CRLF::wrap_async_reader_with_buffer_size(tokio::io::empty(), 0);
}

#[cfg(feature = "tokio")]
#[test]
#[should_panic(expected = "buffer size must be greater than zero")]
fn tokio_writer_rejects_zero_buffer_size() {
    use eolify::TokioExt;

    let _ = CRLF::wrap_async_writer_with_buffer_size(tokio::io::sink(), 0);
}
