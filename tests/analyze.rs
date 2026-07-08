use eolify::{analyze_chunk, analyze_reader, AnalyzeState, LineEndingStats};

#[test]
fn counts_line_ending_styles() {
    let stats = analyze_reader(&b"a\nb\r\nc\rd\n"[..]).unwrap();

    assert_eq!(
        stats,
        LineEndingStats {
            lf: 2,
            crlf: 1,
            cr: 1
        }
    );
    assert_eq!(stats.total(), 4);
    assert!(stats.is_mixed());
}

#[test]
fn handles_crlf_split_across_chunks() {
    let mut state = AnalyzeState::new();
    let mut stats = LineEndingStats::default();

    analyze_chunk(b"a\r", &mut state, &mut stats, false);
    analyze_chunk(b"\nb", &mut state, &mut stats, true);

    assert_eq!(
        stats,
        LineEndingStats {
            lf: 0,
            crlf: 1,
            cr: 0
        }
    );
}

#[test]
fn counts_trailing_lone_cr_at_eof() {
    let stats = analyze_reader(&b"a\r"[..]).unwrap();

    assert_eq!(
        stats,
        LineEndingStats {
            lf: 0,
            crlf: 0,
            cr: 1
        }
    );
}

#[test]
fn no_line_endings_conform_to_lf_and_crlf() {
    let stats = analyze_reader(&b"abc"[..]).unwrap();

    assert!(stats.has_no_line_endings());
    assert!(stats.conforms_to_lf());
    assert!(stats.conforms_to_crlf());
}
