//! `[files] exclude`: glob patterns of paths not to lint, relative to the directory of
//! the configuration file that lists them.

use std::path::{Component, Path, PathBuf};

use globset::{GlobBuilder, GlobSet, GlobSetBuilder};
use kubelint_config::Spanned;

#[derive(Debug)]
pub struct Exclude {
    /// The configuration file's directory, absolute and normalized.
    base: PathBuf,
    /// The working directory, absolute and normalized, to resolve relative paths.
    cwd: PathBuf,
    globs: GlobSet,
}

#[derive(Debug)]
pub struct PatternError {
    /// The pattern's position in the `exclude` list.
    pub index: usize,
    pub message: String,
}

impl Exclude {
    /// Compiles `patterns`, matched against paths relative to `base` (the configuration
    /// file's directory). `cwd` resolves the relative paths given to [`Self::matches`].
    ///
    /// Patterns use `/` as the separator on every platform. `*` and `?` never match a
    /// `/`; `**` matches any number of directories.
    pub fn new(
        base: &Path,
        cwd: &Path,
        patterns: &[Spanned<String>],
    ) -> Result<Self, PatternError> {
        let mut builder = GlobSetBuilder::new();
        for (index, pattern) in patterns.iter().enumerate() {
            let glob = GlobBuilder::new(&pattern.value)
                .literal_separator(true)
                .build()
                .map_err(|err| PatternError {
                    index,
                    message: format!("invalid glob pattern `{}`: {}", pattern.value, err.kind()),
                })?;
            builder.add(glob);
        }
        let globs = builder.build().map_err(|err| PatternError {
            index: 0,
            message: err.to_string(),
        })?;
        let cwd = normalize(cwd);
        Ok(Self {
            base: normalize(&cwd.join(base)),
            cwd,
            globs,
        })
    }

    /// Whether `path` (relative to the working directory, or absolute) is excluded: a
    /// pattern matches it or one of its parent directories below the base. A path
    /// outside the base directory is never excluded.
    pub fn matches(&self, path: &Path) -> bool {
        let full = normalize(&self.cwd.join(path));
        let Ok(relative) = full.strip_prefix(&self.base) else {
            return false;
        };
        relative
            .ancestors()
            .take_while(|p| !p.as_os_str().is_empty())
            .any(|p| self.globs.is_match(p))
    }
}

/// `path` with `.` removed and `..` applied to the component before it, without touching
/// the file system (symbolic links are not resolved).
fn normalize(path: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                out.pop();
            }
            other => out.push(other),
        }
    }
    out
}

#[cfg(test)]
mod tests;
