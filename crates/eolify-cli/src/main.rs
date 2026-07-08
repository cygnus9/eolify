//! CLI orchestration and per-file processing.
//!
//! Argument definitions live in `args`, while output formatting lives in
//! `report`. This module connects those pieces: it expands file inputs, runs
//! one worker per file, classifies files, analyzes line endings, and performs
//! in-place rewrites for `fix`.

use std::{
    ffi::OsString,
    fs::{self, File},
    io::{self, BufRead, BufReader, Read, Write},
    mem,
    path::{Path, PathBuf},
    process,
    sync::{
        atomic::{AtomicU64, Ordering},
        mpsc, Arc, Mutex,
    },
    thread,
};

use content_inspector::{inspect, ContentType};
use eolify::{analyze_reader, LineEndingStats, ReadExt, CRLF, LF};

mod args;
mod report;

use crate::args::{
    parse_args, CheckOptions, Command, FixOptions, GlobalOptions, Input, Target, UnsupportedMode,
};
use crate::report::{emit_reports, FileOutcome, FileReport, OutputMode};

static TEMP_ID: AtomicU64 = AtomicU64::new(0);

/// CLI entry point.
///
/// Exit code `1` is reserved for check failures. Usage, I/O, and internal
/// errors return `2`.
fn main() {
    match run() {
        Ok(code) => process::exit(code),
        Err(err) => {
            eprintln!("eolify: {err}");
            process::exit(2);
        }
    }
}

/// Dispatch the parsed command to the shared file runner.
fn run() -> Result<i32, String> {
    let args = parse_args()?;

    match args.command {
        Command::Fix(mut opts) => {
            let mode = args
                .global_opts
                .unsupported
                .unwrap_or(UnsupportedMode::Skip);
            run_files(
                args.global_opts,
                mem::take(&mut opts.inputs),
                OutputMode::Fix,
                move |path| process_fix(path, &opts, mode),
            )
        }
        Command::Check(mut opts) => {
            let mode = args
                .global_opts
                .unsupported
                .unwrap_or(UnsupportedMode::Skip);
            let output = OutputMode::Check { json: opts.json };
            run_files(
                args.global_opts,
                mem::take(&mut opts.inputs),
                output,
                move |path| process_check(path, &opts, mode),
            )
        }
        Command::Stats(mut opts) => {
            let mode = args
                .global_opts
                .unsupported
                .unwrap_or(UnsupportedMode::Skip);
            let output = OutputMode::Stats {
                json: opts.json,
                files: opts.files,
            };
            run_files(
                args.global_opts,
                mem::take(&mut opts.inputs),
                output,
                move |path| process_stats(path, mode),
            )
        }
    }
}

/// Process selected paths with a fixed per-file operation.
///
/// Positional paths are chained with paths from `--files-from`; the latter are
/// read lazily so processing can begin before the entire path list is loaded.
/// Each worker receives complete file paths only, so a single file is never
/// split across threads.
fn run_files<F>(
    global_opts: GlobalOptions,
    files: Input,
    output_mode: OutputMode,
    process_one: F,
) -> Result<i32, String>
where
    F: Fn(&Path) -> FileReport + Send + Sync + 'static,
{
    if files.files_from.is_none() && files.paths.is_empty() {
        return Err("no input files".to_owned());
    }

    let paths = files.paths.into_iter().map(Result::<_, String>::Ok);
    let paths: Box<dyn Iterator<Item = Result<PathBuf, String>> + Send> =
        if let Some(files_from) = files.files_from {
            Box::new(paths.chain(iter_files_from(&files_from, files.null)?))
        } else {
            Box::new(paths)
        };

    let jobs = global_opts
        .jobs
        .unwrap_or_else(|| thread::available_parallelism().map_or(1, usize::from))
        .max(1);

    let quiet = global_opts.quiet;
    let processor = Arc::new(process_one);
    let paths = Arc::new(Mutex::new(paths));
    let (tx, rx) = mpsc::channel();

    for _ in 0..jobs {
        let paths = Arc::clone(&paths);
        let tx = tx.clone();
        let processor = Arc::clone(&processor);
        thread::spawn(move || loop {
            let path = match paths.lock().expect("path queue poisoned").next() {
                None => break,
                Some(Err(err)) => {
                    let _ = tx.send(FileReport::error(PathBuf::from("<files-from>"), err));
                    break;
                }
                Some(Ok(path)) => path,
            };
            if tx.send(processor(&path)).is_err() {
                break;
            }
        });
    }
    drop(tx);

    let mut had_error = false;
    let mut had_nonconforming = false;
    let mut aggregate = LineEndingStats::default();
    let mut reports = Vec::new();

    for report in rx {
        match &report.outcome {
            Ok(FileOutcome::Nonconforming(stats)) => {
                had_nonconforming = true;
                aggregate_stats(&mut aggregate, stats);
            }
            Ok(FileOutcome::Changed(stats)) | Ok(FileOutcome::Stats(stats)) => {
                aggregate_stats(&mut aggregate, stats);
            }
            Ok(FileOutcome::Skipped(reason)) => {
                if !quiet {
                    eprintln!("skipped {}: {reason}", report.path.display());
                }
            }
            Ok(FileOutcome::Ok) => {}
            Err(err) => {
                had_error = true;
                if !quiet {
                    eprintln!("{}: {err}", report.path.display());
                }
            }
        }
        reports.push(report);
    }

    emit_reports(&reports, aggregate, output_mode, quiet)?;

    if had_error {
        Ok(2)
    } else if had_nonconforming {
        Ok(1)
    } else {
        Ok(0)
    }
}

/// Return an iterator over paths listed in a file or stdin.
///
/// Newline-delimited input is decoded through `BufRead::lines`, while
/// NUL-delimited input preserves arbitrary bytes as an `OsString`. Empty path
/// entries are ignored.
fn iter_files_from(
    path: &Path,
    null: bool,
) -> Result<impl Iterator<Item = Result<PathBuf, String>> + Send, String> {
    let bufread = BufReader::<Box<dyn Read + Send>>::new(if path.eq("-") {
        Box::new(io::stdin())
    } else {
        Box::new(fs::File::open(path).map_err(|err| err.to_string())?)
    });
    let lines: Box<dyn Iterator<Item = io::Result<OsString>> + Send> = if null {
        Box::new(bufread.split(b'\0').map(|result| {
            result.map(|bytes| unsafe { OsString::from_encoded_bytes_unchecked(bytes) })
        }))
    } else {
        Box::new(bufread.lines().map(|result| result.map(OsString::from)))
    };
    Ok(lines.filter_map(|line| match line {
        Ok(path) if path.is_empty() => None,
        Ok(path) => Some(Ok(PathBuf::from(path))),
        Err(err) => Some(Err(err.to_string())),
    }))
}

/// Apply `fix` semantics to one file.
///
/// Unsupported files may be skipped or failed according to `unsupported`.
/// Already-conforming files are left untouched. With `--dry-run`,
/// nonconforming files are reported as changed but not rewritten.
fn process_fix(path: &Path, opts: &FixOptions, unsupported: UnsupportedMode) -> FileReport {
    let outcome = (|| {
        if let Some(reason) = classify_or_skip(path, unsupported)? {
            return Ok(FileOutcome::Skipped(reason));
        }
        let stats = analyze_file(path)?;
        if conforms(stats, opts.target) {
            return Ok(FileOutcome::Ok);
        }
        if opts.dry_run {
            return Ok(FileOutcome::Changed(stats));
        }
        normalize_file(path, opts.target)?;
        Ok(FileOutcome::Changed(stats))
    })();

    FileReport::new(path, outcome)
}

/// Apply `check` semantics to one file.
///
/// Check uses exactly one policy: either a target newline style or mixed-ending
/// detection.
fn process_check(path: &Path, opts: &CheckOptions, unsupported: UnsupportedMode) -> FileReport {
    let outcome = (|| {
        if let Some(reason) = classify_or_skip(path, unsupported)? {
            return Ok(FileOutcome::Skipped(reason));
        }
        let stats = analyze_file(path)?;
        let failed = if opts.mixed {
            stats.is_mixed()
        } else {
            !conforms(
                stats,
                opts.target
                    .expect("check target is present unless --mixed was validated"),
            )
        };
        if failed {
            Ok(FileOutcome::Nonconforming(stats))
        } else {
            Ok(FileOutcome::Ok)
        }
    })();

    FileReport::new(path, outcome)
}

/// Count line ending styles for one file after classification.
fn process_stats(path: &Path, unsupported: UnsupportedMode) -> FileReport {
    let outcome = (|| {
        if let Some(reason) = classify_or_skip(path, unsupported)? {
            return Ok(FileOutcome::Skipped(reason));
        }
        Ok(FileOutcome::Stats(analyze_file(path)?))
    })();

    FileReport::new(path, outcome)
}

/// Classify a path and return a skip reason when policy says it should not be
/// processed.
fn classify_or_skip(path: &Path, unsupported: UnsupportedMode) -> Result<Option<String>, String> {
    match classify(path, unsupported)? {
        Classification::Process => Ok(None),
        Classification::Skip(reason) => Ok(Some(reason)),
    }
}

/// Classification result after applying the unsupported-file policy.
enum Classification {
    Process,
    Skip(String),
}

/// Inspect a sample of the file and apply the unsupported-file policy.
///
/// Regular UTF-8 and UTF-8-with-BOM files are processed. Directories, symlinks,
/// binary files, and UTF-16/UTF-32 files are skipped by default because the CLI
/// operates on ordinary ASCII-compatible text files. `--unsupported process`
/// bypasses the skip policy for binary/encoding classification, but filesystem
/// entries that are not regular files still cannot be processed.
fn classify(path: &Path, mode: UnsupportedMode) -> Result<Classification, String> {
    let metadata = fs::symlink_metadata(path).map_err(|err| err.to_string())?;
    let file_type = metadata.file_type();
    let unsupported_file_type = if file_type.is_dir() {
        Some("directory")
    } else if file_type.is_symlink() {
        Some("symlink")
    } else if !file_type.is_file() {
        Some("not a regular file")
    } else {
        None
    };
    if let Some(reason) = unsupported_file_type {
        return match mode {
            UnsupportedMode::Fail => Err(reason.to_owned()),
            _ => Ok(Classification::Skip(reason.to_owned())),
        };
    }

    let mut file = File::open(path).map_err(|err| err.to_string())?;
    let mut sample = [0; 8192];
    let len = file.read(&mut sample).map_err(|err| err.to_string())?;
    let content_type = inspect(&sample[..len]);

    let unsupported = match content_type {
        ContentType::UTF_8 | ContentType::UTF_8_BOM => return Ok(Classification::Process),
        ContentType::BINARY => Some("binary file"),
        ContentType::UTF_16LE
        | ContentType::UTF_16BE
        | ContentType::UTF_32LE
        | ContentType::UTF_32BE => Some("unsupported encoding"),
    };

    match (mode, unsupported) {
        (UnsupportedMode::Process, _) => Ok(Classification::Process),
        (UnsupportedMode::Fail, Some(reason)) => Err(format!("{reason}: {content_type}")),
        (_, Some(reason)) => Ok(Classification::Skip(format!("{reason}: {content_type}"))),
        _ => Ok(Classification::Process),
    }
}

/// Analyze one file's line endings using the library streaming analyzer.
fn analyze_file(path: &Path) -> Result<LineEndingStats, String> {
    let file = File::open(path).map_err(|err| err.to_string())?;
    analyze_reader(file).map_err(|err| err.to_string())
}

/// Rewrite one file to the requested newline target.
///
/// The caller pre-scans the file and only calls this for files that need a
/// change. Output is written to a temporary sibling path, permissions are
/// copied from the original file, and the temporary file is renamed over the
/// original after a successful write and sync.
fn normalize_file(path: &Path, target: Target) -> Result<(), String> {
    let input = File::open(path).map_err(|err| err.to_string())?;
    let metadata = input.metadata().map_err(|err| err.to_string())?;
    let temp_path = temp_path_for(path)?;
    let mut output = File::create(&temp_path).map_err(|err| err.to_string())?;
    output
        .set_permissions(metadata.permissions())
        .map_err(|err| err.to_string())?;

    match target {
        Target::Lf => {
            let mut reader = input.normalize_newlines(LF);
            io::copy(&mut reader, &mut output).map_err(|err| err.to_string())?;
        }
        Target::Crlf => {
            let mut reader = input.normalize_newlines(CRLF);
            io::copy(&mut reader, &mut output).map_err(|err| err.to_string())?;
        }
    }

    output.flush().map_err(|err| err.to_string())?;
    output.sync_all().map_err(|err| err.to_string())?;
    fs::rename(&temp_path, path).map_err(|err| {
        let _ = fs::remove_file(&temp_path);
        err.to_string()
    })
}

/// Build a temporary sibling path for an in-place rewrite.
fn temp_path_for(path: &Path) -> Result<PathBuf, String> {
    let name = path
        .file_name()
        .ok_or_else(|| "path has no file name".to_owned())?
        .to_string_lossy();
    let id = TEMP_ID.fetch_add(1, Ordering::Relaxed);
    Ok(path.with_file_name(format!(".{name}.eolify.{id}.tmp")))
}

/// Return whether collected stats satisfy the selected newline target.
fn conforms(stats: LineEndingStats, target: Target) -> bool {
    match target {
        Target::Lf => stats.conforms_to_lf(),
        Target::Crlf => stats.conforms_to_crlf(),
    }
}

/// Add one file's stats into the command aggregate.
fn aggregate_stats(total: &mut LineEndingStats, stats: &LineEndingStats) {
    total.lf += stats.lf;
    total.crlf += stats.crlf;
    total.cr += stats.cr;
}
