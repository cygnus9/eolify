use std::{
    io::{Read, Write},
    mem::MaybeUninit,
};

use eolify::{
    helpers::{slice_to_uninit_mut, vec_to_uninit_mut},
    IoExt, Normalize, NormalizeChunk, NormalizeChunkResult, Result, CRLF, LF,
};

struct Identity;

// SAFETY: Checks the buffer length, writes every reported byte, never
// deinitializes bytes, and copies input unchanged (including valid UTF-8).
unsafe impl NormalizeChunk for Identity {
    type State = ();

    fn max_output_size_for_chunk(size: usize, _: Option<&()>, _: bool) -> usize {
        size
    }

    fn normalize_chunk(
        input: &[u8],
        output: &mut [MaybeUninit<u8>],
        _: Option<&()>,
        _: bool,
    ) -> Result<NormalizeChunkResult<()>> {
        if output.len() < input.len() {
            return Err(eolify::Error::OutputBufferTooSmall {
                required: input.len(),
            });
        }
        for (destination, byte) in output.iter_mut().zip(input) {
            destination.write(*byte);
        }
        Ok(NormalizeChunkResult::new(input.len(), None))
    }
}

#[test]
fn audited_custom_normalizer_works_with_blanket_and_io_consumers() {
    let input = "héllo\r\n世界\n🦀";
    assert_eq!(Identity::normalize(input.as_bytes()), input.as_bytes());
    assert_eq!(Identity::normalize_str(input), input);
    let mut reader = Identity::wrap_reader_with_buffer_size(input.as_bytes(), 1);
    let mut output = Vec::new();
    reader.read_to_end(&mut output).unwrap();
    assert_eq!(output, input.as_bytes());
    let mut writer = Identity::wrap_writer_with_buffer_size(Vec::new(), 1);
    writer.write_all(input.as_bytes()).unwrap();
    assert_eq!(writer.finish().unwrap(), input.as_bytes());
}

#[cfg(feature = "futures-io")]
#[test]
fn audited_custom_normalizer_works_with_futures_consumers() {
    use eolify::FuturesIoExt;
    use futures_util::{
        io::{AsyncReadExt, AsyncWriteExt, Cursor},
        FutureExt,
    };

    let input = "héllo\r\n世界\n🦀".as_bytes();
    let mut reader = Identity::wrap_async_reader_with_buffer_size(Cursor::new(input), 1);
    let mut output = Vec::new();
    reader
        .read_to_end(&mut output)
        .now_or_never()
        .unwrap()
        .unwrap();
    assert_eq!(output, input);
    let mut writer = Identity::wrap_async_writer_with_buffer_size(Cursor::new(Vec::new()), 1);
    writer.write_all(input).now_or_never().unwrap().unwrap();
    let output = writer
        .finish()
        .now_or_never()
        .unwrap()
        .unwrap()
        .into_inner();
    assert_eq!(output, input);
}

#[cfg(feature = "tokio")]
#[tokio::test]
async fn audited_custom_normalizer_works_with_tokio_consumers() {
    use eolify::TokioExt;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    let input = "héllo\r\n世界\n🦀".as_bytes();
    let mut reader = Identity::wrap_async_reader_with_buffer_size(input, 1);
    let mut output = Vec::new();
    reader.read_to_end(&mut output).await.unwrap();
    assert_eq!(output, input);
    let mut writer = Identity::wrap_async_writer_with_buffer_size(Vec::new(), 1);
    writer.write_all(input).await.unwrap();
    assert_eq!(writer.finish().await.unwrap(), input);
}

fn check_initialized_prefix<N: NormalizeChunk<State = bool>>() {
    // Read each reported prefix from genuinely uninitialized storage. Miri can
    // detect missing writes here; a pre-zeroed buffer would mask them.
    let alphabet = [b'a', b'\r', b'\n', 0xff];
    for length in 0..=5 {
        for mut pattern in 0..4usize.pow(length) {
            let input: Vec<_> = (0..length)
                .map(|_| {
                    let byte = alphabet[pattern % 4];
                    pattern /= 4;
                    byte
                })
                .collect();
            for state in [None, Some(false), Some(true)] {
                for last in [false, true] {
                    let capacity = N::max_output_size_for_chunk(input.len(), state.as_ref(), last);
                    let mut output = Vec::with_capacity(capacity);
                    // SAFETY: output is empty; no initialized elements can be lost.
                    let status = N::normalize_chunk(
                        &input,
                        unsafe { vec_to_uninit_mut(&mut output) },
                        state.as_ref(),
                        last,
                    )
                    .unwrap();
                    assert!(status.output_len() <= capacity);
                    // SAFETY: N guarantees its reported prefix is initialized and in bounds.
                    unsafe {
                        output.set_len(status.output_len());
                    }
                    assert!(output.iter().all(|byte| alphabet.contains(byte)));
                }
            }
        }
    }
}

#[test]
fn builtins_initialize_every_reported_byte_for_all_chunk_states() {
    check_initialized_prefix::<LF>();
    check_initialized_prefix::<CRLF>();
}

fn check_preservation<N: NormalizeChunk>() {
    let mut output = [0xa5; 16];
    // SAFETY: N preserves initialization, including on error.
    let status = N::normalize_chunk(
        b"a\r\nb",
        unsafe { slice_to_uninit_mut(&mut output) },
        None,
        true,
    )
    .unwrap();
    assert!(output[status.output_len()..]
        .iter()
        .all(|byte| *byte == 0xa5));
    let mut too_small = [0xa5];
    // SAFETY: N preserves initialization, including on error.
    assert!(N::normalize_chunk(
        b"a\r\nb",
        unsafe { slice_to_uninit_mut(&mut too_small) },
        None,
        true
    )
    .is_err());
    assert_eq!(too_small, [0xa5]);
}

#[test]
fn builtins_preserve_initialized_tails_and_error_buffers() {
    check_preservation::<LF>();
    check_preservation::<CRLF>();
}

#[test]
fn builtins_preserve_utf8() {
    let input = "é\r中\n🦀\r\n\0";
    assert_eq!(LF::normalize_str(input), "é\n中\n🦀\n\0");
    assert_eq!(CRLF::normalize_str(input), "é\r\n中\r\n🦀\r\n\0");
}
