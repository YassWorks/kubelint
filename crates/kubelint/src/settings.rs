//! Loading the configuration file and turning it, with the command-line flags, into the
//! rules and fail threshold of a run (`docs/user/configuration.md`).

use std::path::{Path, PathBuf};

use anyhow::Context as _;
use kubelint_config::{Config, Level, Spanned};
use kubelint_diagnostics::Severity;
use kubelint_linter::{EnabledRule, RuleSelection, SelectionError, SelectorList};
use kubelint_output::HumanRenderer;
use kubelint_source::{SourceFile, TextRange};

use crate::exclude::Exclude;

/// Where the configuration comes from.
#[derive(Debug, Clone)]
pub enum ConfigSource {
    /// Look for `kubelint.toml` or `.kubelint.toml` from this directory upwards.
    Discover(PathBuf),
    /// This file, which must exist (`--config`).
    File(PathBuf),
    /// No configuration file (`--isolated`).
    None,
}

/// Rule selection flags from the command line. Each one that is set replaces the
/// configuration file's value.
#[derive(Debug, Clone, Default)]
pub struct Overrides {
    /// `--select`.
    pub select: Option<Vec<String>>,
    /// `--ignore`.
    pub ignore: Option<Vec<String>>,
    /// `--fail-on`.
    pub fail_on: Option<Severity>,
}

/// What a run needs from the configuration.
#[derive(Debug)]
pub struct Settings {
    /// The rules to run, with their severities.
    pub rules: Vec<EnabledRule>,
    /// The lowest severity that fails the run.
    pub fail_on: Severity,
    /// The configuration's `[files] exclude` patterns, if any.
    pub exclude: Option<Exclude>,
}

/// Why the settings could not be built.
#[derive(Debug)]
pub enum SettingsError {
    /// The configuration file cannot be read.
    Io(anyhow::Error),
    /// The configuration or a selector is invalid; the text is ready to print.
    Invalid(String),
}

/// The configuration file's path, relative to `cwd` when it is inside it, so messages
/// stay short.
fn display_path(path: &Path, cwd: &Path) -> PathBuf {
    path.strip_prefix(cwd).unwrap_or(path).to_path_buf()
}

/// Loads the configuration and resolves the rules to run. `cwd` shortens the path shown
/// in error messages; `color` applies to them.
pub fn load(
    source: &ConfigSource,
    overrides: &Overrides,
    cwd: &Path,
    color: bool,
) -> Result<Settings, SettingsError> {
    let path = match source {
        ConfigSource::Discover(dir) => kubelint_config::find(dir),
        ConfigSource::File(path) => Some(path.clone()),
        ConfigSource::None => None,
    };
    let (file, config, dir) = match path {
        None => (None, Config::default(), None),
        Some(path) => {
            let text = std::fs::read_to_string(&path)
                .with_context(|| format!("cannot read the configuration {}", path.display()))
                .map_err(SettingsError::Io)?;
            let file = SourceFile::new(display_path(&path, cwd), text);
            let config = kubelint_config::parse(file.text()).map_err(|err| {
                SettingsError::Invalid(HumanRenderer::new(color).render_error(
                    &file,
                    &err.message,
                    err.range,
                ))
            })?;
            let dir = path.parent().map(Path::to_path_buf);
            (Some(file), config, dir)
        }
    };
    let invalid = |message: &str, range: Option<TextRange>| {
        SettingsError::Invalid(match (&file, range) {
            (Some(file), Some(range)) => {
                HumanRenderer::new(color).render_error(file, message, Some(range))
            }
            // The selector was given on the command line.
            _ => format!("error: {message}\n"),
        })
    };
    let mut settings =
        resolve(&config, overrides).map_err(|(err, range)| invalid(&err.to_string(), range))?;
    let patterns = &config.files.exclude;
    if let (Some(dir), false) = (dir, patterns.is_empty()) {
        let exclude = Exclude::new(&dir, cwd, patterns)
            .map_err(|err| invalid(&err.message, patterns.get(err.index).map(|p| p.range)))?;
        settings.exclude = Some(exclude);
    }
    Ok(settings)
}

/// Combines the configuration and the flags. On error, also returns where the bad
/// selector is in the configuration file, or `None` when it came from a flag.
fn resolve(
    config: &Config,
    overrides: &Overrides,
) -> Result<Settings, (SelectionError, Option<TextRange>)> {
    let lint = &config.lint;
    let file_select = lint.select.as_deref();
    let selection = RuleSelection {
        select: match &overrides.select {
            Some(select) => Some(select.clone()),
            None => file_select.map(values),
        },
        ignore: match &overrides.ignore {
            Some(ignore) => ignore.clone(),
            None => values(&lint.ignore),
        },
        severity: lint
            .severity
            .iter()
            .map(|(key, level)| (key.value.clone(), severity(*level)))
            .collect(),
    };
    let rules = selection.resolve().map_err(|err| {
        let range = match err.list {
            SelectorList::Select if overrides.select.is_none() => {
                file_select.and_then(|s| s.get(err.index)).map(|s| s.range)
            }
            SelectorList::Ignore if overrides.ignore.is_none() => {
                lint.ignore.get(err.index).map(|s| s.range)
            }
            SelectorList::Severity => lint.severity.get(err.index).map(|(key, _)| key.range),
            SelectorList::Select | SelectorList::Ignore => None,
        };
        (err, range)
    })?;
    let fail_on = overrides
        .fail_on
        .or_else(|| lint.fail_on.as_ref().map(|level| severity(level.value)))
        .unwrap_or(Severity::Warning);
    Ok(Settings {
        rules,
        fail_on,
        exclude: None,
    })
}

fn values(list: &[Spanned<String>]) -> Vec<String> {
    list.iter().map(|s| s.value.clone()).collect()
}

/// The diagnostic severity for a configured level.
pub fn severity(level: Level) -> Severity {
    match level {
        Level::Error => Severity::Error,
        Level::Warning => Severity::Warning,
        Level::Info => Severity::Info,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn codes(settings: &Settings) -> Vec<&'static str> {
        settings.rules.iter().map(|r| r.rule.meta().code).collect()
    }

    fn config(text: &str) -> Config {
        kubelint_config::parse(text).expect("valid config")
    }

    #[test]
    fn no_config_and_no_flags_is_the_default_set() {
        let settings = resolve(&Config::default(), &Overrides::default()).expect("valid");
        let defaults: Vec<_> = kubelint_linter::rules::ALL
            .iter()
            .map(|rule| rule.meta())
            .filter(|meta| meta.default_enabled)
            .map(|meta| meta.code)
            .collect();
        assert_eq!(codes(&settings), defaults);
        assert_eq!(settings.fail_on, Severity::Warning);
    }

    #[test]
    fn flags_replace_the_file_lists() {
        let config = config("[lint]\nselect = [\"KL1001\"]\nignore = [\"KL1002\"]\n");
        let from_file = resolve(&config, &Overrides::default()).expect("valid");
        assert_eq!(codes(&from_file), ["KL0001", "KL1001"]);
        let overrides = Overrides {
            select: Some(vec!["KL1001".to_owned(), "KL1002".to_owned()]),
            ignore: Some(Vec::new()),
            fail_on: Some(Severity::Error),
        };
        let from_flags = resolve(&config, &overrides).expect("valid");
        assert_eq!(codes(&from_flags), ["KL0001", "KL1001", "KL1002"]);
        assert_eq!(from_flags.fail_on, Severity::Error);
    }

    #[test]
    fn severity_and_fail_on_come_from_the_file() {
        let config = config("[lint]\nfail-on = \"error\"\n[lint.severity]\nKL1002 = \"info\"\n");
        let settings = resolve(&config, &Overrides::default()).expect("valid");
        assert_eq!(settings.fail_on, Severity::Error);
        let kl1002 = settings
            .rules
            .iter()
            .find(|r| r.rule.meta().code == "KL1002")
            .expect("KL1002 runs");
        assert_eq!(kl1002.severity, Severity::Info);
    }

    #[test]
    fn errors_point_into_the_file_unless_a_flag_was_wrong() {
        let text = "[lint]\nselect = [\"KL1001\", \"secrity\"]\n";
        let (err, range) = resolve(&config(text), &Overrides::default()).expect_err("unknown");
        assert_eq!(err.selector, "secrity");
        let range = range.expect("in the file");
        let at = &text[usize::from(range.start())..usize::from(range.end())];
        assert_eq!(at, "\"secrity\"");

        let overrides = Overrides {
            ignore: Some(vec!["KL9".to_owned()]),
            ..Overrides::default()
        };
        let (err, range) = resolve(&Config::default(), &overrides).expect_err("unknown");
        assert_eq!((err.selector.as_str(), range), ("KL9", None));
    }
}
