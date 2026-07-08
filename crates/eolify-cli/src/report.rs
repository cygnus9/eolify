//! Reporting model and output formatting.
//!
//! The rest of the CLI reports per-file outcomes in terms of `FileReport`.
//! This module turns those reports into human-readable output or JSON, and it
//! keeps presentation details out of the file processing code.

use std::{
    io,
    path::{Path, PathBuf},
};

use eolify::LineEndingStats;
use serde::{
    ser::{SerializeSeq, SerializeStruct},
    Serialize, Serializer,
};

/// Result of processing one input path.
#[derive(Debug)]
pub enum FileOutcome {
    /// The file needed no action and satisfied the requested command.
    Ok,
    /// The file would be or was rewritten, with stats from before rewriting.
    Changed(LineEndingStats),
    /// The file failed a check command, with the line ending stats that failed.
    Nonconforming(LineEndingStats),
    /// Line ending counts collected for a stats command.
    Stats(LineEndingStats),
    /// The file was intentionally ignored, usually because classification marked
    /// it as binary or unsupported.
    Skipped(String),
}

/// Per-file report passed from workers to the output layer.
#[derive(Debug)]
pub struct FileReport {
    /// Input path associated with this result.
    pub path: PathBuf,
    /// Successful command outcome or the error raised while handling the file.
    pub outcome: Result<FileOutcome, String>,
}

impl FileReport {
    /// Build a report for a normal input path.
    pub fn new(path: &Path, outcome: Result<FileOutcome, String>) -> Self {
        Self {
            path: path.to_owned(),
            outcome,
        }
    }

    /// Build a report for an error that is not associated with a concrete input
    /// file, such as an error while reading `--files-from`.
    pub fn error(path: PathBuf, err: String) -> Self {
        Self {
            path,
            outcome: Err(err),
        }
    }
}

/// Output behavior selected by the active command and command flags.
#[derive(Clone, Copy)]
pub enum OutputMode {
    /// Print changed files for fix commands.
    Fix,
    /// Print nonconforming files or JSON check records.
    Check { json: bool },
    /// Print aggregate stats, and optionally per-file stats.
    Stats { json: bool, files: bool },
}

/// Emit command output for collected reports.
///
/// Human-readable stats without `--files` print only aggregate totals. JSON
/// stats always include a `files` array plus `total`. Quiet mode suppresses
/// normal report output but does not change exit-code decisions, which are made
/// by the caller.
pub fn emit_reports(
    reports: &[FileReport],
    aggregate: LineEndingStats,
    output_mode: OutputMode,
    quiet: bool,
) -> Result<(), String> {
    if quiet {
        return Ok(());
    }

    match output_mode {
        OutputMode::Check { json: true } => {
            print_json_reports(reports, false)?;
            return Ok(());
        }
        OutputMode::Stats { json: true, .. } => {
            print_json_stats(reports, aggregate)?;
            return Ok(());
        }
        _ => {}
    }

    for report in reports {
        match &report.outcome {
            Ok(FileOutcome::Changed(_)) => println!("{}", report.path.display()),
            Ok(FileOutcome::Nonconforming(stats)) => {
                println!("{}: {}", report.path.display(), format_stats(*stats));
            }
            Ok(FileOutcome::Stats(stats))
                if matches!(output_mode, OutputMode::Stats { files: true, .. }) =>
            {
                println!("{}: {}", report.path.display(), format_stats(*stats));
            }
            _ => {}
        }
    }

    if matches!(output_mode, OutputMode::Stats { .. }) {
        println!("total: {}", format_stats(aggregate));
    }

    Ok(())
}

/// JSON object for one check report.
#[derive(Serialize)]
struct JsonCheckReport<'a> {
    path: String,
    status: &'a str,
    #[serde(flatten, skip_serializing_if = "Option::is_none")]
    stats: Option<JsonStats>,
}

/// JSON object for one stats report.
#[derive(Serialize)]
struct JsonStatsReport {
    path: String,
    #[serde(flatten)]
    stats: JsonStats,
}

/// Streaming JSON array wrapper for check output.
struct JsonCheckReports<'a> {
    reports: &'a [FileReport],
    include_ok: bool,
}

/// Streaming JSON object wrapper for stats output.
struct JsonStatsOutput<'a> {
    reports: &'a [FileReport],
    total: JsonStats,
}

/// JSON representation of `LineEndingStats`.
#[derive(Serialize)]
struct JsonStats {
    lf: u64,
    crlf: u64,
    cr: u64,
}

impl From<LineEndingStats> for JsonStats {
    fn from(stats: LineEndingStats) -> Self {
        Self {
            lf: stats.lf,
            crlf: stats.crlf,
            cr: stats.cr,
        }
    }
}

/// Write check reports as a JSON array.
fn print_json_reports(reports: &[FileReport], include_ok: bool) -> Result<(), String> {
    let stdout = io::stdout();
    let mut serializer = serde_json::Serializer::new(stdout.lock());
    JsonCheckReports {
        reports,
        include_ok,
    }
    .serialize(&mut serializer)
    .map_err(|err| err.to_string())?;
    println!();
    Ok(())
}

/// Write stats reports as a JSON object containing `files` and `total`.
fn print_json_stats(reports: &[FileReport], aggregate: LineEndingStats) -> Result<(), String> {
    let stdout = io::stdout();
    let mut serializer = serde_json::Serializer::new(stdout.lock());
    JsonStatsOutput {
        reports,
        total: JsonStats::from(aggregate),
    }
    .serialize(&mut serializer)
    .map_err(|err| err.to_string())?;
    println!();
    Ok(())
}

impl Serialize for JsonCheckReports<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut seq = serializer.serialize_seq(None)?;
        for report in self.reports {
            let Some((status, stats)) = json_status(&report.outcome) else {
                continue;
            };
            if !self.include_ok && status == "ok" {
                continue;
            }
            seq.serialize_element(&JsonCheckReport {
                path: report.path.to_string_lossy().into_owned(),
                status,
                stats: stats.map(JsonStats::from),
            })?;
        }
        seq.end()
    }
}

impl Serialize for JsonStatsOutput<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_struct("JsonStatsOutput", 2)?;
        state.serialize_field("files", &JsonStatsFiles(self.reports))?;
        state.serialize_field("total", &self.total)?;
        state.end()
    }
}

/// Streaming JSON array wrapper for stats file entries.
struct JsonStatsFiles<'a>(&'a [FileReport]);

impl Serialize for JsonStatsFiles<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut seq = serializer.serialize_seq(None)?;
        for report in self.0 {
            if let Ok(FileOutcome::Stats(stats)) = &report.outcome {
                seq.serialize_element(&JsonStatsReport {
                    path: report.path.to_string_lossy().into_owned(),
                    stats: JsonStats::from(*stats),
                })?;
            }
        }
        seq.end()
    }
}

/// Convert an internal outcome into the public JSON check status.
fn json_status(
    outcome: &Result<FileOutcome, String>,
) -> Option<(&'static str, Option<LineEndingStats>)> {
    match outcome {
        Ok(FileOutcome::Ok) => Some(("ok", None)),
        Ok(FileOutcome::Nonconforming(stats)) => Some(("nonconforming", Some(*stats))),
        Ok(FileOutcome::Skipped(_)) => Some(("skipped", None)),
        Err(_) => Some(("error", None)),
        _ => None,
    }
}

/// Compact human-readable line ending counters.
fn format_stats(stats: LineEndingStats) -> String {
    format!("lf={} crlf={} cr={}", stats.lf, stats.crlf, stats.cr)
}
