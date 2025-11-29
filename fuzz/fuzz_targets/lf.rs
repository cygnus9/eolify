#![no_main]

use eolify::{helpers::vec_to_uninit_mut, Error, Normalize, LF};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if data.is_empty() {
        return;
    }

    // Control byte layout (data[0]):
    // bit 0: preceded_by_cr
    // bit 1: is_last_chunk
    // bit 2: undersize output buffer (exercise OutputBufferTooSmall)
    let control = data[0];
    let preceded_by_cr = (control & 0x01) != 0;
    let is_last_chunk = (control & 0x02) != 0;
    let undersize = (control & 0x04) != 0;
    let payload = &data[1..];

    let mut buf_len = payload.len();
    if undersize && buf_len > 0 {
        buf_len = buf_len.saturating_sub(1);
    }
    let mut out = Vec::with_capacity(buf_len);
    match (
        LF::normalize_chunk(
            payload,
            vec_to_uninit_mut(&mut out),
            preceded_by_cr,
            is_last_chunk,
        ),
        undersize && !payload.is_empty(),
    ) {
        (Ok(_), false) | (Err(Error::OutputBufferTooSmall { .. }), true) => {
            // all good
        }
        (a, b) => {
            panic!("Unexpected result from normalize_chunk: ({a:?}, {b})");
        }
    }
});
