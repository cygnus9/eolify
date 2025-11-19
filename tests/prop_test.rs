use eolify::{Normalize, CRLF};
use proptest::{arbitrary::any, collection::vec, prop_assert, proptest, test_runner::Config};

proptest! {
    #![proptest_config(Config::with_cases(25000))]

    #[test]
    fn normalize_chunk_chunk_properties(data in vec(any::<u8>(), 0..256), split_byte in any::<u8>()) {
        let split = (split_byte as usize).min(data.len());
        let (a, b) = data.split_at(split);

        // allocate safe upper-bound buffers
        let mut out1 = vec![0u8; a.len() * 2];
        let status1 = CRLF::normalize_chunk(a, &mut out1, false, false)
            .expect("output buffer too small for first chunk");
        out1.truncate(status1.output_len());

        let mut out2 = vec![0u8; b.len() * 2 + 1];
        let status2 = CRLF::normalize_chunk(b, &mut out2, status1.ended_with_cr(), true)
            .expect("output buffer too small for second chunk");
        out2.truncate(status2.output_len());

        let combined = [out1.as_slice(), out2.as_slice()].concat();

        // Basic length properties
        prop_assert!(status1.output_len() >= a.len(), "status1.output_len < a.len()");
        prop_assert!(status2.output_len() >= b.len(), "status2.output_len < b.len()");
        prop_assert!(combined.len() >= data.len(), "combined.len() < data.len()");

        // out1: no lone LF/CR, except possible trailing CR which must match status1.ended_with_cr
        if !out1.is_empty() {
            for i in 0..out1.len() {
                let c = out1[i];
                if c == b'\r' {
                    if i == out1.len() - 1 {
                        prop_assert!(status1.ended_with_cr(), "out1 ends with CR but status1.ended_with_cr is false");
                    } else {
                        prop_assert!(out1[i + 1] == b'\n', "found lone CR in out1 at {}", i);
                    }
                } else if c == b'\n' {
                    prop_assert!(i > 0 && out1[i - 1] == b'\r', "found lone LF in out1 at {}", i);
                }
            }
        } else {
            // empty out1 must reflect ended_with_cr == false (since we passed preceded_by_cr=false)
            prop_assert!(!status1.ended_with_cr() || a.is_empty(), "empty out1 with ended_with_cr true");
        }

        // out2: no lone LF/CR, except possible leading LF if status1.ended_with_cr is true,
        // and possible trailing CR if status2.ended_with_cr is true.
        if !out2.is_empty() {
            for i in 0..out2.len() {
                let c = out2[i];
                if c == b'\r' {
                    if i == out2.len() - 1 {
                        prop_assert!(status2.ended_with_cr(), "out2 ends with CR but status2.ended_with_cr is false");
                    } else {
                        prop_assert!(out2[i + 1] == b'\n', "found lone CR in out2 at {}", i);
                    }
                } else if c == b'\n' {
                    if i == 0 {
                        prop_assert!(status1.ended_with_cr(), "out2 starts with LF but status1.ended_with_cr is false");
                    } else {
                        prop_assert!(out2[i - 1] == b'\r', "found lone LF in out2 at {}", i);
                    }
                }
            }
        } else {
            // empty out2 is allowed; if status1.ended_with_cr was true, the LF may have been consumed
            // into out2 in which case out2 would not be empty, so no further assertion here.
        }

        // combined must contain only CRLF pairs (no lone CR or lone LF)
        for i in 0..combined.len() {
            let c = combined[i];
            if c == b'\r' {
                prop_assert!(i + 1 < combined.len() && combined[i + 1] == b'\n', "found lone CR in combined at {}", i);
            } else if c == b'\n' {
                prop_assert!(i > 0 && combined[i - 1] == b'\r', "found lone LF in combined at {}", i);
            }
        }
    }

    #[test]
    fn normalize_chunk_idempotent(data in vec(any::<u8>(), 0..256)) {
        // First normalization
        let mut out1 = vec![0u8; data.len() * 2 + 1];
        let status1 = CRLF::normalize_chunk(&data, &mut out1, false, true)
            .expect("output buffer too small for first normalization");
        out1.truncate(status1.output_len());

        // out1 must contain only CRLF pairs (no lone CR or lone LF)
        for i in 0..out1.len() {
            let c = out1[i];
            if c == b'\r' {
                prop_assert!(i + 1 < out1.len() && out1[i + 1] == b'\n', "found lone CR in out1 at {}", i);
            } else if c == b'\n' {
                prop_assert!(i > 0 && out1[i - 1] == b'\r', "found lone LF in out1 at {}", i);
            }
        }

        // Second normalization
        let mut out2 = vec![0u8; out1.len() * 2 + 1];
        let status2 = CRLF::normalize_chunk(&out1, &mut out2, false, true)
            .expect("output buffer too small for second normalization");
        out2.truncate(status2.output_len());

        // The second normalization should be identical to the first
        prop_assert!(out1 == out2, "second normalization differs from first");
    }
}
