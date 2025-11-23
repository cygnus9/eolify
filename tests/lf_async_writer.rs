#![cfg(any(feature = "futures-io", feature = "tokio"))]

macro_rules! dual_test {
    ($name:ident, $body:block) => {
        mod $name {
            use eolify::LF;

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
    let mut writer = LF::wrap_async_writer_with_buffer_size(Vec::new(), 4);
    writer.write_all(b"foo\r").await.unwrap();
    writer.write_all(b"\nbar").await.unwrap();
    let out = writer.finish().await.unwrap();
    assert_eq!(out, b"foo\nbar".to_vec());
});

dual_test!(crlf_split_across_three_chunks, {
    let mut writer = LF::wrap_async_writer_with_buffer_size(Vec::new(), 4);
    writer.write_all(b"foo\r").await.unwrap();
    writer.flush().await.unwrap();
    writer.write_all(b"\nbar").await.unwrap();
    let out = writer.finish().await.unwrap();
    assert_eq!(out, b"foo\nbar".to_vec());
});

dual_test!(lone_lf_in_first_chunk_kept_as_lf, {
    let mut writer = LF::wrap_async_writer_with_buffer_size(Vec::new(), 5);
    writer.write_all(b"line1\n").await.unwrap();
    writer.write_all(b"line2").await.unwrap();
    let out = writer.finish().await.unwrap();
    assert_eq!(out, b"line1\nline2".to_vec());
});

dual_test!(multiple_crs_and_crlf_mixed_across_boundaries, {
    let mut writer = LF::wrap_async_writer_with_buffer_size(Vec::new(), 1);
    writer.write_all(b"\r").await.unwrap();
    writer.write_all(b"\r\n").await.unwrap();
    let out = writer.finish().await.unwrap();
    assert_eq!(out, b"\n\n".to_vec());
});

dual_test!(trailing_cr_at_eof_emits_lf, {
    let mut writer = LF::wrap_async_writer_with_buffer_size(Vec::new(), 16);
    writer.write_all(b"foo\r").await.unwrap();
    let out = writer.finish().await.unwrap();
    assert_eq!(out, b"foo\n".to_vec());
});
