//! `[files] exclude`: glob patterns of paths not to lint, relative to the directory of
//! the configuration file that lists them.

use std::path::{Component, Path, PathBuf};

use globset::{GlobBuilder, GlobSet, GlobSetBuilder};
use kubelint_config::Spanned;

/// Compiled `exclude` patterns.
#[derive(Debug)]
pub struct Exclude {
    /// The configuration file's directory, absolute and normalized.
    base: PathBuf,
    /// The working directory, absolute and normalized, to resolve relative paths.
    cwd: PathBuf,
    globs: GlobSet,
}

/// A pattern that is not a valid glob.
#[derive(Debug)]
pub struct PatternError {
    /// Its position in the `exclude` list.
    pub index: usize,
    /// What is wrong with it.
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
mod tests {
    use kubelint_source::TextRange;

    use super::*;

    fn patterns(globs: &[&str]) -> Vec<Spanned<String>> {
        globs
            .iter()
            .map(|g| Spanned {
                value: (*g).to_owned(),
                range: TextRange::default(),
            })
            .collect()
    }

    /// An absolute path that works on every platform.
    fn root() -> PathBuf {
        std::env::temp_dir().join("kubelint-exclude")
    }

    fn exclude(base: &str, cwd: &str, globs: &[&str]) -> Exclude {
        let root = root();
        Exclude::new(&root.join(base), &root.join(cwd), &patterns(globs)).expect("valid globs")
    }

    #[test]
    fn patterns_are_relative_to_the_config_directory() {
        let exclude = exclude(
            "project",
            "project/deploy",
            &["deploy/generated/**", "*.tmp.yaml"],
        );
        assert!(exclude.matches(Path::new("generated/a.yaml")));
        assert!(exclude.matches(Path::new("./generated/sub/b.yaml")));
        assert!(!exclude.matches(Path::new("web.yaml")));
        // `*` does not cross `/`: only files directly in the base directory.
        assert!(exclude.matches(Path::new("../x.tmp.yaml")));
        assert!(!exclude.matches(Path::new("x.tmp.yaml")));
    }

    #[test]
    fn a_directory_pattern_excludes_everything_below_it() {
        let exclude = exclude("p", "p", &["vendor", "**/testdata"]);
        assert!(exclude.matches(Path::new("vendor/a/b.yaml")));
        assert!(exclude.matches(Path::new("src/x/testdata/c.yaml")));
        assert!(!exclude.matches(Path::new("vendored/a.yaml")));
    }

    #[test]
    fn paths_outside_the_base_are_never_excluded() {
        let exclude = exclude("p/sub", "p", &["**"]);
        assert!(exclude.matches(Path::new("sub/a.yaml")));
        assert!(!exclude.matches(Path::new("a.yaml")));
        assert!(!exclude.matches(&root().join("elsewhere/a.yaml")));
    }

    #[test]
    fn invalid_patterns_are_reported_with_their_index() {
        let root = root();
        let err = Exclude::new(&root, &root, &patterns(&["ok/**", "[a"])).expect_err("invalid");
        assert_eq!(err.index, 1);
        assert!(
            err.message.starts_with("invalid glob pattern `[a`: "),
            "{}",
            err.message
        );
    }
}
