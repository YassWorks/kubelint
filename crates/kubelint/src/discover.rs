//! Turns the paths given on the command line into the files to lint.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use anyhow::{Context as _, bail};

/// One thing to lint.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Input {
    /// Standard input, requested with `-`.
    Stdin,
    /// A file on disk.
    File(PathBuf),
}

/// Expands `paths` into inputs, in order, without duplicates.
///
/// - `-` is standard input (at most once).
/// - A file is linted whatever its extension: naming it is a clear request.
/// - A directory is searched recursively for `*.yaml` and `*.yml` files, skipping hidden
///   files and whatever `.gitignore`, `.ignore` and global git excludes list (also outside
///   a git repository). The files of one directory are sorted by path.
/// - A path that does not exist is an error.
pub fn inputs(paths: &[PathBuf]) -> anyhow::Result<Vec<Input>> {
    let mut inputs = Vec::new();
    let mut seen = HashSet::new();
    let mut add = |inputs: &mut Vec<Input>, file: PathBuf| {
        if seen.insert(file.clone()) {
            inputs.push(Input::File(file));
        }
    };
    for path in paths {
        if path.as_os_str() == "-" {
            if inputs.contains(&Input::Stdin) {
                bail!("`-` (standard input) can only be given once");
            }
            inputs.push(Input::Stdin);
        } else if path.is_dir() {
            for file in yaml_files(path)? {
                add(&mut inputs, file);
            }
        } else if path.exists() {
            add(&mut inputs, path.clone());
        } else {
            bail!("{}: no such file or directory", path.display());
        }
    }
    Ok(inputs)
}

fn yaml_files(dir: &Path) -> anyhow::Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    for entry in ignore::WalkBuilder::new(dir).require_git(false).build() {
        let entry = entry.with_context(|| format!("cannot search {}", dir.display()))?;
        let is_file = entry.file_type().is_some_and(|t| t.is_file());
        if is_file && is_yaml(entry.path()) {
            // `kubelint check .` should print `deploy/web.yaml`, not `./deploy/web.yaml`.
            let path = entry.into_path();
            files.push(match path.strip_prefix(".") {
                Ok(relative) if dir == Path::new(".") => relative.to_path_buf(),
                _ => path,
            });
        }
    }
    files.sort();
    Ok(files)
}

fn is_yaml(path: &Path) -> bool {
    path.extension()
        .is_some_and(|ext| ext.eq_ignore_ascii_case("yaml") || ext.eq_ignore_ascii_case("yml"))
}
