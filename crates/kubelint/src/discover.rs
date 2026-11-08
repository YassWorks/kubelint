//! Turns the paths given on the command line into the files to lint.

use std::collections::{HashMap, HashSet};
use std::path::{Component, Path, PathBuf};

use anyhow::{Context as _, bail};

use crate::exclude::Exclude;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Input {
    /// Standard input, requested with `-`.
    Stdin,
    File(PathBuf),
}

#[derive(Debug, Default)]
pub struct Discovery {
    /// The inputs to lint, in order, without duplicates.
    pub inputs: Vec<Input>,
    /// Helm charts whose files were skipped, in the order they were first met.
    pub skipped_charts: Vec<SkippedChart>,
}

impl Discovery {
    pub fn files_skipped(&self) -> usize {
        self.skipped_charts.iter().map(|chart| chart.files).sum()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SkippedChart {
    /// The chart directory (the one holding `Chart.yaml`), as the user would write it.
    pub dir: PathBuf,
    pub files: usize,
}

/// Expands `paths` into inputs, in order, without duplicates.
///
/// - `-` is standard input (at most once).
/// - A file is linted whatever its extension: naming it is a clear request.
/// - A directory is searched recursively for `*.yaml` and `*.yml` files, skipping hidden
///   files and whatever `.gitignore`, `.ignore` and global git excludes list (also outside
///   a git repository). The files of one directory are sorted by path.
/// - Helm chart files are skipped, even when named: a chart's `Chart.yaml` and everything
///   under its `templates/` directory. Templates are not valid YAML until `helm template`
///   renders them, and `Chart.yaml` is not a Kubernetes object.
/// - Files matching the configuration's `[files] exclude` patterns are left out, even when
///   named, so tools that pass file names (pre-commit) respect them too.
/// - A path that does not exist is an error.
pub fn inputs(paths: &[PathBuf], exclude: Option<&Exclude>) -> anyhow::Result<Discovery> {
    let mut discovery = Discovery::default();
    let mut seen = HashSet::new();
    let mut helm = HelmDetector::new();
    let mut add = |discovery: &mut Discovery, file: PathBuf| {
        if exclude.is_some_and(|exclude| exclude.matches(&file)) || !seen.insert(file.clone()) {
            return;
        }
        match helm.chart_of(&file) {
            Some(dir) => match discovery.skipped_charts.iter_mut().find(|c| c.dir == dir) {
                Some(chart) => chart.files += 1,
                None => discovery
                    .skipped_charts
                    .push(SkippedChart { dir, files: 1 }),
            },
            None => discovery.inputs.push(Input::File(file)),
        }
    };
    for path in paths {
        if path.as_os_str() == "-" {
            if discovery.inputs.contains(&Input::Stdin) {
                bail!("`-` (standard input) can only be given once");
            }
            discovery.inputs.push(Input::Stdin);
        } else if path.is_dir() {
            for file in yaml_files(path)? {
                add(&mut discovery, file);
            }
        } else if path.exists() {
            add(&mut discovery, path.clone());
        } else {
            bail!("{}: no such file or directory", path.display());
        }
    }
    Ok(discovery)
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

/// Finds the Helm chart a file belongs to, remembering which directories are charts.
///
/// A chart is a directory holding a `Chart.yaml` file (Helm accepts no other name). Its
/// files kubelint skips are `Chart.yaml` itself and everything under `templates/`, at any
/// depth. Other chart files (`values.yaml`, `crds/`) are linted as usual: they hold no
/// templates, and documents that are not Kubernetes objects are ignored anyway.
struct HelmDetector {
    /// The working directory, to look above relative paths (`kubelint check .` run inside
    /// `templates/`). `None` if it cannot be read, then only the path itself is searched.
    cwd: Option<PathBuf>,
    /// Whether a directory (absolute when `cwd` is known) holds a `Chart.yaml`.
    is_chart: HashMap<PathBuf, bool>,
}

impl HelmDetector {
    fn new() -> Self {
        Self {
            cwd: std::env::current_dir().ok(),
            is_chart: HashMap::new(),
        }
    }

    /// The chart directory if `file` is a chart file kubelint skips, written relative to
    /// the same base as `file`.
    fn chart_of(&mut self, file: &Path) -> Option<PathBuf> {
        // Without `.` components, `file` and `full` end in the same components, so the
        // n-th ancestor of one is the n-th ancestor of the other.
        let file: PathBuf = file
            .components()
            .filter(|c| *c != Component::CurDir)
            .collect();
        let full = match &self.cwd {
            Some(cwd) => cwd.join(&file),
            None => file.clone(),
        };
        let chart_depth = if full.file_name().is_some_and(|name| name == "Chart.yaml") {
            Some(1)
        } else {
            self.templates_chart_depth(&full)
        }?;
        // Show the chart relative to the same base as `file`. A relative `file` has
        // `known` ancestors before the empty path; a chart above that is `..`, `../..`.
        let known = file.ancestors().count() - 1;
        let dir = match file.ancestors().nth(chart_depth) {
            Some(dir) if dir.as_os_str().is_empty() => PathBuf::from("."),
            Some(dir) => dir.to_path_buf(),
            None => (known..chart_depth).map(|_| "..").collect(),
        };
        Some(dir)
    }

    /// For a file under `<chart>/templates/`, how many levels up the chart directory is.
    fn templates_chart_depth(&mut self, full: &Path) -> Option<usize> {
        for (depth, dir) in full.ancestors().enumerate().skip(1) {
            if dir.file_name().is_some_and(|name| name == "templates") {
                let chart = dir.parent()?;
                if self.is_chart(chart) {
                    return Some(depth + 1);
                }
            }
        }
        None
    }

    fn is_chart(&mut self, dir: &Path) -> bool {
        *self
            .is_chart
            .entry(dir.to_path_buf())
            .or_insert_with(|| dir.join("Chart.yaml").is_file())
    }
}

#[cfg(test)]
mod tests;
