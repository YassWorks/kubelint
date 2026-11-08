use std::collections::BTreeMap;
use std::fmt;
use std::ops::Range;

use kubelint_source::{TextRange, TextSize};
use serde::Deserialize;

/// A parsed configuration file. Every field is optional; an empty file is valid.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Config {
    pub lint: LintConfig,
    pub files: FilesConfig,
}

/// The `[lint]` table: which rules run and when the run fails.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LintConfig {
    /// `select`: the rules to run. `None` means the default set.
    pub select: Option<Vec<Spanned<String>>>,
    /// `ignore`: rules removed from the selection.
    pub ignore: Vec<Spanned<String>>,
    /// `fail-on`: the lowest severity that fails the run.
    pub fail_on: Option<Spanned<Level>>,
    /// `[lint.severity]`: severity overrides, keyed by rule code or name, sorted by key.
    pub severity: Vec<(Spanned<String>, Level)>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FilesConfig {
    /// `exclude`: glob patterns of paths not to lint.
    pub exclude: Vec<Spanned<String>>,
}

/// A severity as written in configuration: `error`, `warning` or `info`.
///
/// This crate does not depend on `kubelint_diagnostics`; the caller maps it to
/// `Severity`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Level {
    Error,
    Warning,
    Info,
}

/// A value and the byte range of its text in the configuration file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Spanned<T> {
    pub value: T,
    /// Where it was written, quotes included.
    pub range: TextRange,
}

/// A configuration file that cannot be used: invalid TOML, an unknown key or a value of
/// the wrong type.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigError {
    /// What is wrong, for example ``unknown field `selct`, expected one of ...``.
    pub message: String,
    pub range: Option<TextRange>,
}

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for ConfigError {}

/// Parses the text of a configuration file. Unknown keys are errors, never ignored.
///
/// ```
/// let error = kubelint_config::parse("[lint]\nselct = []\n").unwrap_err();
/// assert!(error.message.starts_with("unknown field `selct`"));
/// assert_eq!(error.range.map(|r| u32::from(r.start())), Some(7));
/// ```
pub fn parse(text: &str) -> Result<Config, ConfigError> {
    let raw: RawConfig = toml::from_str(text).map_err(|err| ConfigError {
        message: err.message().trim_end().to_owned(),
        range: err.span().and_then(text_range),
    })?;
    let lint = raw.lint.unwrap_or_default();
    let files = raw.files.unwrap_or_default();
    Ok(Config {
        lint: LintConfig {
            select: lint.select.map(spanned_all),
            ignore: spanned_all(lint.ignore.unwrap_or_default()),
            fail_on: lint.fail_on.map(spanned),
            severity: lint
                .severity
                .unwrap_or_default()
                .into_iter()
                .map(|(key, level)| (spanned(key), level))
                .collect(),
        },
        files: FilesConfig {
            exclude: spanned_all(files.exclude.unwrap_or_default()),
        },
    })
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawConfig {
    lint: Option<RawLint>,
    files: Option<RawFiles>,
}

#[derive(Default, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
struct RawLint {
    select: Option<Vec<toml::Spanned<String>>>,
    ignore: Option<Vec<toml::Spanned<String>>>,
    fail_on: Option<toml::Spanned<Level>>,
    severity: Option<BTreeMap<toml::Spanned<String>, Level>>,
}

#[derive(Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawFiles {
    exclude: Option<Vec<toml::Spanned<String>>>,
}

fn spanned<T>(value: toml::Spanned<T>) -> Spanned<T> {
    let range = text_range(value.span()).unwrap_or_default();
    Spanned {
        value: value.into_inner(),
        range,
    }
}

fn spanned_all<T>(values: Vec<toml::Spanned<T>>) -> Vec<Spanned<T>> {
    values.into_iter().map(spanned).collect()
}

/// `None` for a range past 4 GiB, which `TextSize` cannot hold.
fn text_range(range: Range<usize>) -> Option<TextRange> {
    let start = TextSize::try_from(range.start).ok()?;
    let end = TextSize::try_from(range.end).ok()?;
    Some(TextRange::new(start, end))
}
