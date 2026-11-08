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

#[derive(Debug, Clone)]
pub enum ConfigSource {
    /// Look for `kubelint.toml` or `.kubelint.toml` from the working directory upwards.
    Discover,
    /// This file, which must exist (`--config`).
    File(PathBuf),
    /// No configuration file (`--isolated`).
    None,
}

/// Rule selection flags from the command line. Each one that is set replaces the
/// configuration file's value.
#[derive(Debug, Clone, Default)]
pub struct Overrides {
    pub select: Option<Vec<String>>,
    pub ignore: Option<Vec<String>>,
    pub fail_on: Option<Severity>,
}

#[derive(Debug)]
pub struct Settings {
    pub rules: Vec<EnabledRule>,
    pub fail_on: Severity,
    pub exclude: Option<Exclude>,
}

#[derive(Debug)]
pub enum SettingsError {
    Io(anyhow::Error),
    /// The configuration or a selector is invalid; the text is ready to print.
    Invalid(String),
}

/// Loads the configuration and resolves the rules to run. Paths in error messages are
/// shown relative to `cwd`; `color` applies to them.
pub fn load(
    source: ConfigSource,
    overrides: &Overrides,
    cwd: &Path,
    color: bool,
) -> Result<Settings, SettingsError> {
    let human = HumanRenderer::new(color);
    let path = match source {
        ConfigSource::Discover => kubelint_config::find(cwd),
        ConfigSource::File(path) => Some(path),
        ConfigSource::None => None,
    };
    let (file, config, dir) = match path {
        None => (None, Config::default(), None),
        Some(path) => {
            let text = std::fs::read_to_string(&path)
                .with_context(|| format!("cannot read the configuration {}", path.display()))
                .map_err(SettingsError::Io)?;
            let file = SourceFile::new(path.strip_prefix(cwd).unwrap_or(&path), text);
            let config = kubelint_config::parse(file.text()).map_err(|err| {
                SettingsError::Invalid(human.render_error(&file, &err.message, err.range))
            })?;
            let dir = path.parent().map(Path::to_path_buf);
            (Some(file), config, dir)
        }
    };
    let invalid = |message: &str, range: Option<TextRange>| {
        SettingsError::Invalid(match (&file, range) {
            (Some(file), Some(range)) => human.render_error(file, message, Some(range)),
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
        select: overrides.select.clone().or_else(|| file_select.map(values)),
        ignore: overrides
            .ignore
            .clone()
            .unwrap_or_else(|| values(&lint.ignore)),
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

fn severity(level: Level) -> Severity {
    match level {
        Level::Error => Severity::Error,
        Level::Warning => Severity::Warning,
        Level::Info => Severity::Info,
    }
}

#[cfg(test)]
mod tests;
