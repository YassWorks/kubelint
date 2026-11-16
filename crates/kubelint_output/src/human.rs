use std::ops::Range;
use std::time::Duration;

use annotate_snippets::{Annotation, AnnotationKind, Group, Level, Renderer, Snippet};
use kubelint_diagnostics::{Diagnostic, Label, Severity};
use kubelint_source::{FileId, SourceFile, TextRange, TextSize};

use crate::{file, path_string};

/// Counts for the summary line printed after the diagnostics.
#[derive(Debug, Clone, Copy, Default)]
pub struct Summary {
    pub files_checked: usize,
    pub objects_checked: usize,
    /// Files found but not linted (Helm chart templates).
    pub files_skipped: usize,
    /// `None` leaves the timing out, so the output is the same on every run.
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
    /// let summary = Summary { files_checked: 1, objects_checked: 1, files_skipped: 0, elapsed: None };
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
        let renderer = self.renderer();
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

    /// Renders an error that stops the run before linting, such as an invalid
    /// configuration file: a snippet of `file` pointing at `range`, or just the path and
    /// the message when the position is unknown.
    ///
    /// ```
    /// use kubelint_output::HumanRenderer;
    /// use kubelint_source::{SourceFile, TextRange};
    ///
    /// let file = SourceFile::new("kubelint.toml", "[lnt]\n");
    /// let range = TextRange::new(1.into(), 4.into());
    /// let out = HumanRenderer::new(false).render_error(&file, "unknown field `lnt`", Some(range));
    /// assert!(out.starts_with("error: unknown field `lnt`\n --> kubelint.toml:1:2\n"));
    /// ```
    pub fn render_error(
        &self,
        file: &SourceFile,
        message: &str,
        range: Option<TextRange>,
    ) -> String {
        let path = path_string(file);
        let Some(range) = range else {
            return format!("error: {path}: {message}\n");
        };
        let report = [
            Group::with_title(Level::ERROR.primary_title(message)).element(
                Snippet::source(file.text())
                    .path(path)
                    .fold(true)
                    .annotation(AnnotationKind::Primary.span(span(file, range))),
            ),
        ];
        let mut out = self.renderer().render(&report);
        out.push('\n');
        out
    }

    fn renderer(self) -> Renderer {
        if self.color {
            Renderer::styled()
        } else {
            Renderer::plain()
        }
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
    let mut by_file: Vec<(FileId, Vec<(&'a Label, AnnotationKind)>)> = Vec::new();
    for (label, kind) in labels {
        match by_file.iter_mut().find(|(id, _)| *id == label.file) {
            Some((_, labels)) => labels.push((label, kind)),
            None => by_file.push((label.file, vec![(label, kind)])),
        }
    }
    let mut group = Group::with_title(
        level(diagnostic.severity)
            .primary_title(diagnostic.message.as_str())
            .id(diagnostic.code),
    );
    for (id, labels) in by_file {
        if let Some(file) = file(files, id) {
            let spans: Vec<_> = labels.iter().map(|(l, _)| span(file, l.range)).collect();
            let (text, first_line, offset) = excerpt(file, &spans);
            let annotations = labels.into_iter().zip(spans).map(|((label, kind), span)| {
                annotation(label, kind, span.start - offset..span.end - offset)
            });
            group = group.element(
                Snippet::source(text)
                    .line_start(first_line)
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

fn annotation(label: &Label, kind: AnnotationKind, span: Range<usize>) -> Annotation<'_> {
    let annotation = kind.span(span);
    match &label.message {
        Some(message) => annotation.label(message.as_str()),
        None => annotation,
    }
}

/// The whole lines that `spans` touch: their text, the number of the first one and the
/// byte offset where they start. Rendering only these lines keeps each diagnostic from
/// costing a pass over the whole file.
fn excerpt<'a>(file: &'a SourceFile, spans: &[Range<usize>]) -> (&'a str, usize, usize) {
    let text = file.text();
    let index = file.line_index();
    // annotate-snippets shows an offset at the end of a text that ends with a newline
    // on its last line, so the excerpt must hold that line.
    let line = |offset: usize| {
        let offset = offset - usize::from(offset == text.len() && text.ends_with('\n'));
        let offset = TextSize::new(u32::try_from(offset).unwrap_or(u32::MAX));
        index.line_col(text, offset).line
    };
    let line_start = |line: u32| index.line_range(text, line).map(|r| usize::from(r.start()));
    let lo = spans.iter().map(|s| s.start).min().unwrap_or(0);
    let hi = spans.iter().map(|s| s.end).max().unwrap_or(0);
    let first = line(lo);
    let start = line_start(first).unwrap_or(lo).min(lo);
    let end = line_start(line(hi) + 1).unwrap_or(text.len()).max(hi);
    match text.get(start..end) {
        Some(lines) => (lines, usize::try_from(first).unwrap_or(1), start),
        None => (text, 1, 0),
    }
}

/// `range` as byte indexes clamped to the file.
fn span(file: &SourceFile, range: TextRange) -> Range<usize> {
    let len = file.text().len();
    let start = usize::from(range.start()).min(len);
    start..usize::from(range.end()).clamp(start, len)
}

/// `Found 2 errors and 1 warning. Checked 3 files and 12 objects in 42ms.`, or `All
/// checks passed.` first when there is nothing to report. Skipped files, if any, come
/// before the timing: `Checked 3 files and 12 objects, skipped 2 files in 42ms.`
pub(crate) fn summary_line(diagnostics: &[Diagnostic], summary: &Summary) -> String {
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
    let found = match parts.as_slice() {
        [] => "All checks passed.".to_owned(),
        [one] => format!("Found {one}."),
        [init @ .., last] => format!("Found {} and {last}.", init.join(", ")),
    };
    let skipped = match summary.files_skipped {
        0 => String::new(),
        n => format!(", skipped {}", plural(n, "file")),
    };
    let elapsed = summary
        .elapsed
        .map(|elapsed| format!(" in {}", format_duration(elapsed)))
        .unwrap_or_default();
    format!(
        "{found} Checked {} and {}{skipped}{elapsed}.",
        plural(summary.files_checked, "file"),
        plural(summary.objects_checked, "object"),
    )
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
