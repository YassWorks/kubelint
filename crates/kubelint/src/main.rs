//! The `kubelint` command-line interface.
//!
//! A thin layer over the library crates: parses arguments, discovers files, calls the linter,
//! renders output and picks the exit code. Business logic belongs in the library crates.
//! See `docs/architecture.md`.

mod discover;
mod exclude;
mod settings;

use std::io::{IsTerminal as _, Read as _, Write as _};
use std::path::PathBuf;
use std::process::ExitCode;
use std::time::Instant;

use anyhow::Context as _;
use clap::{Parser, Subcommand, ValueEnum};
use kubelint_diagnostics::Severity;
use kubelint_linter::{Linter, rules};
use kubelint_output::{HumanRenderer, Summary};
use kubelint_source::SourceFile;

use crate::discover::{Input, SkippedChart};
use crate::settings::{ConfigSource, Overrides, SettingsError};

/// A fast, friendly linter for Kubernetes manifests.
#[derive(Debug, Parser)]
#[command(version, about)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Lint Kubernetes manifests.
    Check(CheckArgs),
}

#[derive(Debug, clap::Args)]
struct CheckArgs {
    /// Files or directories to lint, or `-` for standard input. Directories are searched
    /// for `*.yaml` and `*.yml` files, honoring `.gitignore`.
    #[arg(default_value = ".")]
    paths: Vec<PathBuf>,

    /// When to color the output.
    #[arg(long, value_enum, default_value_t = ColorChoice::Auto)]
    color: ColorChoice,

    /// Use this configuration file instead of looking for `kubelint.toml` or
    /// `.kubelint.toml` in the current directory and its parents.
    #[arg(long, value_name = "PATH")]
    config: Option<PathBuf>,

    /// Ignore every configuration file.
    #[arg(long, conflicts_with = "config")]
    isolated: bool,

    /// Rules to run, replacing the configuration's `select`: rule codes, code prefixes
    /// (`KL2`), categories (`security`) or rule names, separated by commas.
    #[arg(long, value_name = "RULES", value_delimiter = ',')]
    select: Option<Vec<String>>,

    /// Rules not to run, replacing the configuration's `ignore`. Same selectors as
    /// `--select`.
    #[arg(long, value_name = "RULES", value_delimiter = ',')]
    ignore: Option<Vec<String>>,

    /// The lowest severity that makes the run fail (exit code 1), replacing the
    /// configuration's `fail-on`. Defaults to `warning`.
    #[arg(long, value_enum, value_name = "SEVERITY")]
    fail_on: Option<FailOn>,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum FailOn {
    /// Fail on errors only.
    Error,
    /// Fail on warnings and errors.
    Warning,
    /// Fail on any finding.
    Info,
}

impl From<FailOn> for Severity {
    fn from(level: FailOn) -> Self {
        match level {
            FailOn::Error => Self::Error,
            FailOn::Warning => Self::Warning,
            FailOn::Info => Self::Info,
        }
    }
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum ColorChoice {
    /// Color when writing to a terminal and `NO_COLOR` is not set.
    Auto,
    /// Always color.
    Always,
    /// Never color.
    Never,
}

/// Exit codes (ADR 0003).
const EXIT_FINDINGS: u8 = 1;
const EXIT_FAILURE: u8 = 2;

fn main() -> ExitCode {
    let cli = match Cli::try_parse() {
        Ok(cli) => cli,
        Err(err) => {
            // `--help` and `--version` are not failures.
            let code = if err.use_stderr() { EXIT_FAILURE } else { 0 };
            let _ = err.print();
            return ExitCode::from(code);
        }
    };
    let result = match cli.command {
        Command::Check(args) => check(&args),
    };
    match result {
        Ok(code) => code,
        Err(err) => {
            eprintln!("error: {err:#}");
            ExitCode::from(EXIT_FAILURE)
        }
    }
}

fn check(args: &CheckArgs) -> anyhow::Result<ExitCode> {
    let start = Instant::now();
    let color = match args.color {
        ColorChoice::Always => true,
        ColorChoice::Never => false,
        ColorChoice::Auto => {
            std::io::stdout().is_terminal() && std::env::var_os("NO_COLOR").is_none()
        }
    };
    let cwd = std::env::current_dir().context("cannot read the current directory")?;
    let source = if args.isolated {
        ConfigSource::None
    } else if let Some(path) = &args.config {
        ConfigSource::File(path.clone())
    } else {
        ConfigSource::Discover(cwd.clone())
    };
    let overrides = Overrides {
        select: args.select.clone(),
        ignore: args.ignore.clone(),
        fail_on: args.fail_on.map(Severity::from),
    };
    let settings = match settings::load(&source, &overrides, &cwd, color) {
        Ok(settings) => settings,
        Err(SettingsError::Io(err)) => return Err(err),
        Err(SettingsError::Invalid(message)) => {
            eprint!("{message}");
            return Ok(ExitCode::from(EXIT_FAILURE));
        }
    };
    let discovery = discover::inputs(&args.paths, settings.exclude.as_ref())?;
    if let Some(note) = helm_note(&discovery.skipped_charts) {
        eprintln!("{note}");
    }
    let files = read_inputs(&discovery.inputs)?;
    let linter = Linter::new(settings.rules);
    let result = linter.lint(&files);
    let summary = Summary {
        files_checked: files.len(),
        objects_checked: result.objects_checked,
        files_skipped: discovery.files_skipped(),
        elapsed: Some(start.elapsed()),
    };
    let output = HumanRenderer::new(color).render(&files, &result.diagnostics, rule_name, &summary);
    let mut stdout = std::io::stdout().lock();
    stdout
        .write_all(output.as_bytes())
        .and_then(|()| stdout.flush())
        .context("cannot write the output")?;

    let failed = kubelint_linter::fails(&result.diagnostics, settings.fail_on);
    Ok(ExitCode::from(if failed { EXIT_FINDINGS } else { 0 }))
}

/// Reads every input. A file that cannot be read stops the run with exit code 2.
fn read_inputs(inputs: &[Input]) -> anyhow::Result<Vec<SourceFile>> {
    inputs
        .iter()
        .map(|input| match input {
            Input::Stdin => {
                let mut text = String::new();
                std::io::stdin()
                    .read_to_string(&mut text)
                    .context("cannot read standard input")?;
                Ok(SourceFile::new("<stdin>", text))
            }
            Input::File(path) => {
                let text = std::fs::read_to_string(path)
                    .with_context(|| format!("cannot read {}", path.display()))?;
                Ok(SourceFile::new(path.clone(), text))
            }
        })
        .collect()
}

/// One note for all skipped Helm charts, telling the user how to lint them anyway.
fn helm_note(charts: &[SkippedChart]) -> Option<String> {
    let files: usize = charts.iter().map(|chart| chart.files).sum();
    let files = if files == 1 {
        "1 file".to_owned()
    } else {
        format!("{files} files")
    };
    let dir = |chart: &SkippedChart| chart.dir.to_string_lossy().replace('\\', "/");
    let (what, chart) = match charts {
        [] => return None,
        [one] => (format!("the Helm chart `{}`", dir(one)), dir(one)),
        many => {
            let dirs: Vec<String> = many.iter().map(|c| format!("`{}`", dir(c))).collect();
            let what = format!("{} Helm charts ({})", many.len(), dirs.join(", "));
            (what, "<chart>".to_owned())
        }
    };
    Some(format!(
        "note: skipped {files} of {what}; kubelint checks rendered manifests, not templates\n  \
         = help: run `helm template {chart} | kubelint check -`"
    ))
}

fn rule_name(code: &str) -> Option<&'static str> {
    rules::ALL
        .iter()
        .map(|rule| rule.meta())
        .find(|meta| meta.code == code)
        .map(|meta| meta.name)
}
