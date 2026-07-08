//! Command-line argument definitions.
//!
//! This module owns the user-facing CLI shape and help text. It intentionally
//! does not validate file contents or decide command behavior; parsed values are
//! passed to `main` for orchestration and per-file processing.

use std::path::PathBuf;

use clap::{error::ErrorKind, Parser};

/// Parse process arguments into the structured command model.
pub fn parse_args() -> Result<Args, String> {
    let args = Args::try_parse().map_err(|err| match err.kind() {
        ErrorKind::DisplayHelp | ErrorKind::DisplayVersion => {
            let _ = err.print();
            std::process::exit(0);
        }
        _ => err.to_string(),
    })?;
    args.validate()?;
    Ok(args)
}

/// Fast newline normalization for files and streams.
#[derive(clap::Parser)]
pub struct Args {
    /// Options accepted before or after any subcommand.
    #[command(flatten)]
    pub global_opts: GlobalOptions,
    /// Operation to perform on the selected files.
    #[clap(subcommand)]
    pub command: Command,
}

impl Args {
    fn validate(&self) -> Result<(), String> {
        if let Command::Check(opts) = &self.command {
            match (opts.target.is_some(), opts.mixed) {
                (true, true) => {
                    Err("check accepts either --to <TARGET> or --mixed, not both".to_owned())
                }
                (false, false) => Err("check requires one of: --to <TARGET>, --mixed".to_owned()),
                _ => Ok(()),
            }
        } else {
            Ok(())
        }
    }
}

/// Options shared by all commands.
#[derive(clap::Args)]
pub struct GlobalOptions {
    /// Maximum number of files to process concurrently.
    ///
    /// Each worker processes whole files; a single file is never split across
    /// workers. Defaults to the platform's available parallelism.
    #[arg(long, short, global = true)]
    pub jobs: Option<usize>,
    /// How to handle files that look binary or use unsupported encodings.
    ///
    /// `skip` ignores those files, `fail` turns them into errors, and `process`
    /// treats them as byte streams anyway.
    #[arg(long, short, global = true)]
    pub unsupported: Option<UnsupportedMode>,
    /// Suppress normal status output.
    ///
    /// Errors still determine the exit code.
    #[arg(long, short, global = true)]
    pub quiet: bool,
}

/// Policy for files that are not ordinary ASCII-compatible text.
#[derive(clap::ValueEnum, Clone, Copy)]
#[clap(rename_all = "kebab_case")]
pub enum UnsupportedMode {
    /// Ignore unsupported files and keep processing other inputs.
    Skip,
    /// Treat unsupported files as errors.
    Fail,
    /// Process unsupported files as raw byte streams.
    Process,
}

/// Top-level CLI command.
#[derive(clap::Subcommand)]
pub enum Command {
    /// Normalize files in place.
    Fix(FixOptions),
    /// Check whether files match a newline policy.
    Check(CheckOptions),
    /// Count line ending styles.
    Stats(StatsOptions),
}

/// File selection options shared by all commands.
#[derive(clap::Args, Default)]
pub struct Input {
    /// Read additional file paths from PATH, or from stdin when PATH is `-`.
    #[arg(long, short)]
    pub files_from: Option<PathBuf>,
    /// Read `--files-from` input as NUL-delimited paths.
    ///
    /// This is intended for commands such as `git ls-files -z` and preserves
    /// paths containing whitespace or newlines.
    #[arg(long, short = '0')]
    pub null: bool,
    /// Files to process.
    #[arg(value_name = "FILE")]
    pub paths: Vec<PathBuf>,
}

/// Target newline policy.
#[derive(clap::ValueEnum, Clone, Copy)]
#[clap(rename_all = "kebab_case")]
pub enum Target {
    /// Unix line endings (`\n`).
    Lf,
    /// Windows line endings (`\r\n`).
    Crlf,
}

/// Options for `fix`.
#[derive(clap::Parser)]
pub struct FixOptions {
    /// Files selected directly or through `--files-from`.
    #[command(flatten)]
    pub inputs: Input,
    /// Newline style to write.
    #[arg(long = "to")]
    pub target: Target,
    /// Report files that would change without rewriting them.
    #[arg(long)]
    pub dry_run: bool,
}

/// Options for `check`.
#[derive(clap::Parser)]
pub struct CheckOptions {
    /// Files selected directly or through `--files-from`.
    #[command(flatten)]
    pub inputs: Input,
    /// Newline style files must conform to.
    #[arg(long = "to")]
    pub target: Option<Target>,
    /// Fail files that contain more than one line ending style.
    #[arg(long)]
    pub mixed: bool,
    /// Emit machine-readable JSON.
    #[arg(long)]
    pub json: bool,
}

/// Options for `stats`.
#[derive(clap::Parser)]
pub struct StatsOptions {
    /// Files selected directly or through `--files-from`.
    #[command(flatten)]
    pub inputs: Input,
    /// Print one human-readable stats line per file in addition to totals.
    #[arg(long)]
    pub files: bool,
    /// Emit machine-readable JSON.
    #[arg(long)]
    pub json: bool,
}
