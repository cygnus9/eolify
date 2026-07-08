use std::io::{self, Read};

use memchr::memchr2;

use crate::types::{CR, LF};

/// Counts line ending styles in a byte stream.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct LineEndingStats {
    pub lf: u64,
    pub crlf: u64,
    pub cr: u64,
}

impl LineEndingStats {
    /// Returns the total number of line endings found.
    #[must_use]
    pub fn total(&self) -> u64 {
        self.lf + self.crlf + self.cr
    }

    /// Returns `true` when no line endings were found.
    #[must_use]
    pub fn has_no_line_endings(&self) -> bool {
        self.total() == 0
    }

    /// Returns `true` when more than one line ending style was found.
    #[must_use]
    pub fn is_mixed(&self) -> bool {
        [self.lf, self.crlf, self.cr]
            .into_iter()
            .filter(|count| *count > 0)
            .count()
            > 1
    }

    /// Returns `true` when all line endings are LF, or when there are no line endings.
    #[must_use]
    pub fn conforms_to_lf(&self) -> bool {
        self.crlf == 0 && self.cr == 0
    }

    /// Returns `true` when all line endings are CRLF, or when there are no line endings.
    #[must_use]
    pub fn conforms_to_crlf(&self) -> bool {
        self.lf == 0 && self.cr == 0
    }
}

/// Analyzer state that carries a trailing `\r` across chunk boundaries.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct AnalyzeState {
    pending_cr: bool,
}

impl AnalyzeState {
    /// Construct a fresh analyzer state.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
}

/// Analyze a chunk of bytes and update line ending counts.
pub fn analyze_chunk(
    input: &[u8],
    state: &mut AnalyzeState,
    stats: &mut LineEndingStats,
    is_last_chunk: bool,
) {
    let mut scan_pos = 0;

    if state.pending_cr {
        if input.first() == Some(&LF) {
            stats.crlf += 1;
            scan_pos = 1;
        } else {
            stats.cr += 1;
        }
        state.pending_cr = false;
    }

    while let Some(i) = memchr2(CR, LF, &input[scan_pos..]).map(|i| i + scan_pos) {
        match (input[i], input.get(i + 1).copied()) {
            (CR, Some(LF)) => {
                stats.crlf += 1;
                scan_pos = i + 2;
            }
            (CR, Some(_)) => {
                stats.cr += 1;
                scan_pos = i + 1;
            }
            (CR, None) => {
                if is_last_chunk {
                    stats.cr += 1;
                } else {
                    state.pending_cr = true;
                }
                return;
            }
            (LF, _) => {
                stats.lf += 1;
                scan_pos = i + 1;
            }
            _ => unreachable!("memchr2 only returns CR or LF positions"),
        }
    }

    if is_last_chunk && state.pending_cr {
        stats.cr += 1;
        state.pending_cr = false;
    }
}

/// Analyze all bytes read from `reader`.
pub fn analyze_reader<R: Read>(mut reader: R) -> io::Result<LineEndingStats> {
    let mut buf = [0; 64 * 1024];
    let mut state = AnalyzeState::new();
    let mut stats = LineEndingStats::default();

    loop {
        let len = reader.read(&mut buf)?;
        if len == 0 {
            analyze_chunk(&[], &mut state, &mut stats, true);
            return Ok(stats);
        }
        analyze_chunk(&buf[..len], &mut state, &mut stats, false);
    }
}
