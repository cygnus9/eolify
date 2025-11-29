use eolify::{helpers::vec_to_uninit_mut, Normalize, LF};
use proptest::{arbitrary::any, collection::vec, prop_assert, proptest, test_runner::Config};

proptest! {
    #![proptest_config(Config::with_cases(25000))]

    #[test]
    fn normalize_chunk_chunk_properties(data in vec(any::<u8>(), 0..256), split_byte in any::<u8>()) {
        let split = (split_byte as usize).min(data.len());
        let (a, b) = data.split_at(split);

        // allocate safe upper-bound buffers
        let mut out1 = Vec::with_capacity(a.len());
        let status1 = LF::normalize_chunk(a,vec_to_uninit_mut( &mut out1), false, false)
            .expect("output buffer too small for first chunk");
        unsafe {
            out1.set_len(status1.output_len());
        }

        let mut out2 = Vec::with_capacity(b.len() * 2 + 1);
        let status2 = LF::normalize_chunk(b,vec_to_uninit_mut( &mut out2), status1.ended_with_cr(), true)
            .expect("output buffer too small for second chunk");
        unsafe {
            out2.set_len(status2.output_len());
        }

        let combined = [out1.as_slice(), out2.as_slice()].concat();

        // Basic length properties
        prop_assert!(status1.output_len() <= a.len(), "status1.output_len > a.len()");
        prop_assert!(status2.output_len() <= b.len(), "status2.output_len > b.len()");
        prop_assert!(combined.len() <= data.len(), "combined.len() > data.len()");

        // out1: no CR whatsoever.
        if !out1.is_empty() {
            for i in 0..out1.len() {
                prop_assert!(out1[i] != b'\r', "found a CR in out1 at {}", i);
            }
        } else {
            // empty out1 must reflect ended_with_cr == false (since we passed preceded_by_cr=false)
            prop_assert!(!status1.ended_with_cr() || a.is_empty(), "empty out1 with ended_with_cr true");
        }

        // out2: no CR whatsoever.
        if !out2.is_empty() {
            for i in 0..out2.len() {
                prop_assert!(out2[i] != b'\r', "found a CR in out2 at {}", i);
            }
        } else {
            // empty out2 is allowed; if status1.ended_with_cr was true, the LF may have been consumed
            // into out2 in which case out2 would not be empty, so no further assertion here.
        }
    }

    #[test]
    fn normalize_chunk_idempotent(data in vec(any::<u8>(), 0..256)) {
        // First normalization
        let mut out1 = Vec::with_capacity(data.len() * 2 + 1);
        let status1 = LF::normalize_chunk(&data,vec_to_uninit_mut( &mut out1), false, true)
            .expect("output buffer too small for first normalization");
        unsafe {
            out1.set_len(status1.output_len());
        }

        // out1 must not contain any CR
        for i in 0..out1.len() {
            prop_assert!(out1[i] != b'\r', "found a CR in out1 at {}", i);
        }

        // Second normalization
        let mut out2 = Vec::with_capacity(out1.len() * 2 + 1);
        let status2 = LF::normalize_chunk(&out1,vec_to_uninit_mut( &mut out2), false, true)
            .expect("output buffer too small for second normalization");
        unsafe {
            out2.set_len(status2.output_len());
        }

        // The second normalization should be identical to the first
        prop_assert!(out1 == out2, "second normalization differs from first");
    }
}
