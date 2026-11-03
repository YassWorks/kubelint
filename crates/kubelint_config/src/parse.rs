//! Parsing a configuration file.

use std::collections::BTreeMap;
use std::fmt;
use std::ops::Range;

use kubelint_source::{TextRange, TextSize};
use serde::Deserialize;

/// A parsed configuration file. Every field is optional; an empty file is valid.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Config {
    /// The `[lint]` table.
    pub lint: LintConfig,
    /// The `[files]` table.
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

/// The `[files]` table.
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
    /// `"error"`.
    Error,
    /// `"warning"`.
    Warning,
    /// `"info"`.
    Info,
}

/// A value and the byte range of its text in the configuration file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Spanned<T> {
    /// The value.
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
    /// Where, when known.
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

#[cfg(test)]
mod tests {
    use super::*;

    /// The text at `range` in `text`.
    fn at(text: &str, range: TextRange) -> &str {
        &text[usize::from(range.start())..usize::from(range.end())]
    }

    #[test]
    fn empty_file_is_the_default() {
        assert_eq!(parse(""), Ok(Config::default()));
        assert_eq!(parse("# nothing\n[lint]\n[files]\n"), Ok(Config::default()));
    }

    #[test]
    fn every_key_with_its_position() {
        let text = r#"
[lint]
select = ["correctness", "KL3001"]
ignore = ["KL2004"]
fail-on = "info"

[lint.severity]
KL3003 = "error"
missing-kind = "warning"

[files]
exclude = ["vendor/**"]
"#;
        let config = parse(text).expect("valid");
        let select = config.lint.select.expect("select is set");
        let select: Vec<_> = select
            .iter()
            .map(|s| (s.value.as_str(), at(text, s.range)))
            .collect();
        assert_eq!(
            select,
            [("correctness", "\"correctness\""), ("KL3001", "\"KL3001\"")]
        );
        assert_eq!(config.lint.ignore[0].value, "KL2004");
        let fail_on = config.lint.fail_on.expect("fail-on is set");
        assert_eq!(
            (fail_on.value, at(text, fail_on.range)),
            (Level::Info, "\"info\"")
        );
        let severity: Vec<_> = config
            .lint
            .severity
            .iter()
            .map(|(key, level)| (at(text, key.range), *level))
            .collect();
        assert_eq!(
            severity,
            [("KL3003", Level::Error), ("missing-kind", Level::Warning)]
        );
        assert_eq!(at(text, config.files.exclude[0].range), "\"vendor/**\"");
    }

    #[test]
    fn empty_select_is_kept_apart_from_no_select() {
        let config = parse("[lint]\nselect = []\n").expect("valid");
        assert_eq!(config.lint.select, Some(Vec::new()));
    }

    /// The error message and the text it points at.
    fn error(text: &str) -> (String, &str) {
        let error = parse(text).expect_err("invalid");
        let range = error.range.expect("has a position");
        (error.message, at(text, range))
    }

    #[test]
    fn unknown_keys_are_errors() {
        let (message, at) = error("[lint]\nselect = []\nfail_on = \"error\"\n");
        assert!(
            message.starts_with("unknown field `fail_on`, expected one of"),
            "{message}"
        );
        assert_eq!(at, "fail_on");
        let (message, at) = error("[lnt]\n");
        assert!(message.starts_with("unknown field `lnt`"), "{message}");
        assert_eq!(at, "lnt");
        let (message, _) = error("[files]\ninclude = []\n");
        assert!(message.starts_with("unknown field `include`"), "{message}");
    }

    #[test]
    fn wrong_values_are_errors() {
        let (message, at) = error("[lint]\nfail-on = \"warn\"\n");
        assert!(message.starts_with("unknown variant `warn`"), "{message}");
        assert_eq!(at, "\"warn\"");
        let (message, at) = error("[lint]\nselect = \"security\"\n");
        assert!(message.contains("expected a sequence"), "{message}");
        assert_eq!(at, "\"security\"");
        let (message, at) = error("[lint.severity]\nKL3003 = \"fatal\"\n");
        assert!(message.starts_with("unknown variant `fatal`"), "{message}");
        assert_eq!(at, "\"fatal\"");
    }

    #[test]
    fn invalid_toml_is_an_error() {
        let (message, _) = error("[lint\n");
        assert!(!message.is_empty());
    }
}
