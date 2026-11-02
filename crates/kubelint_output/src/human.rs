//! The human renderer: rustc-style snippets and a summary line.

use std::fmt::Write as _;
use std::time::Duration;

use annotate_snippets::{Annotation, AnnotationKind, Group, Level, Renderer, Snippet};
use kubelint_diagnostics::{Diagnostic, Label, Severity};
use kubelint_source::{FileId, SourceFile};

/// Counts for the summary line printed after the diagnostics.
#[derive(Debug, Clone, Copy, Default)]
pub struct Summary {
    /// How many files were linted.
    pub files_checked: usize,
    /// How many Kubernetes objects were linted.
    pub objects_checked: usize,
    /// How long the run took. `None` leaves the timing out, which keeps test output
    /// stable.
    pub elapsed: Option<Duration>,
}

/// Renders diagnostics for a terminal.
///
/// `files` are the files of the run, indexed by [`FileId`](kubelint_source::FileId).
/// `rule_name` maps a rule code to its name (`KL2001` to `privileged-container`); the
/// renderer cannot look it up itself because rules live in a higher layer.
#[derive(Debug, Clone, Copy)]
pub struct HumanRenderer {
    color: bool,
}

impl HumanRenderer {
    /// A renderer that colors its output with ANSI escape codes when `color` is true.
    pub fn new(color: bool) -> Self {
        Self { color }
    }

    /// Renders every diagnostic, then the summary line.
    ///
    /// ```
    /// use kubelint_diagnostics::{Diagnostic, Label, Severity};
    /// use kubelint_output::{HumanRenderer, Summary};
    /// use kubelint_source::{FileId, SourceFile, TextRange};
    ///
    /// let files = [SourceFile::new("pod.yaml", "kind: Pod\n")];
    /// let range = TextRange::new(0.into(), 9.into());
    /// let diagnostics = [Diagnostic::new(
    ///     "KL1001",
    ///     Severity::Error,
    ///     "`Pod` object has no `apiVersion`",
    ///     Label::new(FileId(0), range),
    /// )];
    /// let summary = Summary { files_checked: 1, objects_checked: 1, elapsed: None };
    /// let out = HumanRenderer::new(false).render(&files, &diagnostics, |_| None, &summary);
    /// assert!(out.starts_with("error[KL1001]: `Pod` object has no `apiVersion`\n"));
    /// assert!(out.ends_with("Found 1 error. Checked 1 file and 1 object.\n"));
    /// ```
    pub fn render<'n>(
        &self,
        files: &[SourceFile],
        diagnostics: &[Diagnostic],
        rule_name: impl Fn(&str) -> Option<&'n str>,
        summary: &Summary,
    ) -> String {
        let renderer = if self.color {
            Renderer::styled()
        } else {
            Renderer::plain()
        };
        let mut out = String::new();
        for diagnostic in diagnostics {
            let report = [group(files, diagnostic, rule_name(diagnostic.code))];
            out.push_str(&renderer.render(&report));
            out.push_str("\n\n");
        }
        out.push_str(&summary_line(diagnostics, summary));
        out.push('\n');
        out
    }
}

fn level(severity: Severity) -> Level<'static> {
    match severity {
        Severity::Error => Level::ERROR,
        Severity::Warning => Level::WARNING,
        Severity::Info => Level::INFO,
    }
}

/// One diagnostic as an `annotate-snippets` group: the title, one snippet per file with
/// that file's labels (the primary label's file first), the help and the rule name.
fn group<'a>(
    files: &'a [SourceFile],
    diagnostic: &'a Diagnostic,
    rule_name: Option<&'a str>,
) -> Group<'a> {
    let labels = std::iter::once((&diagnostic.primary, AnnotationKind::Primary)).chain(
        diagnostic
            .secondary
            .iter()
            .map(|label| (label, AnnotationKind::Context)),
    );
    let mut by_file: Vec<(FileId, Vec<Annotation<'a>>)> = Vec::new();
    for (label, kind) in labels {
        let Some(file) = file(files, label.file) else {
            continue;
        };
        let annotation = annotation(file, label, kind);
        match by_file.iter_mut().find(|(id, _)| *id == label.file) {
            Some((_, annotations)) => annotations.push(annotation),
            None => by_file.push((label.file, vec![annotation])),
        }
    }
    let mut group = Group::with_title(
        level(diagnostic.severity)
            .primary_title(diagnostic.message.as_str())
            .id(diagnostic.code),
    );
    for (id, annotations) in by_file {
        if let Some(file) = file(files, id) {
            group = group.element(
                Snippet::source(file.text())
                    .path(path_string(file))
                    .fold(true)
                    .annotations(annotations),
            );
        }
    }
    if let Some(help) = &diagnostic.help {
        group = group.element(Level::HELP.message(help.as_str()));
    }
    if let Some(name) = rule_name {
        group = group.element(Level::NOTE.message(format!("rule `{name}`")));
    }
    group
}

/// The file with this id. A label pointing at a file that is not part of the run (a bug
/// elsewhere) is left out rather than panicking.
fn file(files: &[SourceFile], id: FileId) -> Option<&SourceFile> {
    files.get(usize::try_from(id.0).ok()?)
}

/// The annotation for `label`, its range clamped to the file.
fn annotation<'a>(file: &SourceFile, label: &'a Label, kind: AnnotationKind) -> Annotation<'a> {
    let len = file.text().len();
    let start = usize::from(label.range.start()).min(len);
    let end = usize::from(label.range.end()).clamp(start, len);
    let annotation = kind.span(start..end);
    match &label.message {
        Some(message) => annotation.label(message.as_str()),
        None => annotation,
    }
}

/// The file's path with `/` separators, so output looks the same on every platform.
fn path_string(file: &SourceFile) -> String {
    file.path().to_string_lossy().replace('\\', "/")
}

/// `Found 2 errors and 1 warning. Checked 3 files and 12 objects in 42ms.`, or `All
/// checks passed.` first when there is nothing to report.
fn summary_line(diagnostics: &[Diagnostic], summary: &Summary) -> String {
    let parts: Vec<String> = [
        (Severity::Error, "error"),
        (Severity::Warning, "warning"),
        (Severity::Info, "info"),
    ]
    .into_iter()
    .filter_map(|(severity, noun)| {
        let n = diagnostics
            .iter()
            .filter(|d| d.severity == severity)
            .count();
        (n > 0).then(|| plural(n, noun))
    })
    .collect();
    let mut line = match parts.as_slice() {
        [] => "All checks passed.".to_owned(),
        [one] => format!("Found {one}."),
        [init @ .., last] => format!("Found {} and {last}.", init.join(", ")),
    };
    let _ = write!(
        line,
        " Checked {} and {}",
        plural(summary.files_checked, "file"),
        plural(summary.objects_checked, "object")
    );
    if let Some(elapsed) = summary.elapsed {
        let _ = write!(line, " in {}", format_duration(elapsed));
    }
    line.push('.');
    line
}

fn plural(n: usize, noun: &str) -> String {
    // "info" is a mass noun: "2 info", not "2 infos".
    if n == 1 || noun == "info" {
        format!("{n} {noun}")
    } else {
        format!("{n} {noun}s")
    }
}

/// Milliseconds below 10 seconds (`42ms`), seconds with one decimal above (`12.3s`).
fn format_duration(elapsed: Duration) -> String {
    if elapsed < Duration::from_secs(10) {
        format!("{}ms", elapsed.as_millis())
    } else {
        format!("{:.1}s", elapsed.as_secs_f64())
    }
}
