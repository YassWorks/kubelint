//! The JSON renderer: one document with a schema version, the findings and the counts.
//!
//! The schema is a public contract, documented in `docs/user/output-formats.md` and
//! ADR 0013. Adding a field is allowed; renaming or removing one needs a new `version`.

use kubelint_diagnostics::{Diagnostic, Label, Severity};
use kubelint_source::{SourceFile, TextSize};
use serde::Serialize;

use crate::Summary;
use crate::human::path_string;

/// The `version` field of the JSON output. Bumped only for breaking changes.
pub const JSON_SCHEMA_VERSION: u32 = 1;

/// Renders diagnostics as a JSON document for scripts.
///
/// `files` and `rule_name` work as for [`HumanRenderer`](crate::HumanRenderer). The
/// summary's `elapsed` time is left out, so the same input always gives the same output.
#[derive(Debug, Clone, Copy, Default)]
pub struct JsonRenderer;

impl JsonRenderer {
    /// Renders every diagnostic and the counts as pretty-printed JSON, ending with a
    /// newline.
    ///
    /// ```
    /// use kubelint_diagnostics::{Diagnostic, Label, Severity};
    /// use kubelint_output::{JsonRenderer, Summary};
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
    /// let out = JsonRenderer.render(&files, &diagnostics, |_| Some("missing-api-version"), &summary);
    /// let value: serde_json::Value = serde_json::from_str(&out).unwrap();
    /// assert_eq!(value["version"], 1);
    /// assert_eq!(value["diagnostics"][0]["location"]["path"], "pod.yaml");
    /// assert_eq!(value["summary"]["by_severity"]["error"], 1);
    /// ```
    pub fn render<'n>(
        &self,
        files: &[SourceFile],
        diagnostics: &[Diagnostic],
        rule_name: impl Fn(&str) -> Option<&'n str>,
        summary: &Summary,
    ) -> String {
        let count = |severity| {
            diagnostics
                .iter()
                .filter(|d| d.severity == severity)
                .count()
        };
        let output = Output {
            version: JSON_SCHEMA_VERSION,
            diagnostics: diagnostics
                .iter()
                .map(|d| JsonDiagnostic {
                    code: d.code,
                    name: rule_name(d.code),
                    severity: d.severity.as_str(),
                    message: &d.message,
                    help: d.help.as_deref(),
                    location: location(files, &d.primary),
                    related: d.secondary.iter().map(|l| location(files, l)).collect(),
                })
                .collect(),
            summary: JsonSummary {
                by_severity: BySeverity {
                    error: count(Severity::Error),
                    warning: count(Severity::Warning),
                    info: count(Severity::Info),
                },
                files_checked: summary.files_checked,
                objects_checked: summary.objects_checked,
                files_skipped: summary.files_skipped,
            },
        };
        // Serializing plain structs of strings and integers cannot fail.
        let mut out = serde_json::to_string_pretty(&output).unwrap_or_default();
        out.push('\n');
        out
    }
}

fn location<'a>(files: &[SourceFile], label: &'a Label) -> Location<'a> {
    let file = files.get(label.file.0 as usize);
    let position = |offset: TextSize| match file {
        Some(file) => {
            let pos = file.line_index().line_col(file.text(), offset);
            Position {
                line: pos.line,
                column: pos.utf8_col,
                offset: u32::from(offset),
            }
        }
        None => Position {
            line: 0,
            column: 0,
            offset: u32::from(offset),
        },
    };
    Location {
        path: file.map(path_string).unwrap_or_default(),
        start: position(label.range.start()),
        end: position(label.range.end()),
        message: label.message.as_deref(),
    }
}

#[derive(Serialize)]
struct Output<'a> {
    version: u32,
    diagnostics: Vec<JsonDiagnostic<'a>>,
    summary: JsonSummary,
}

#[derive(Serialize)]
struct JsonDiagnostic<'a> {
    code: &'a str,
    name: Option<&'a str>,
    severity: &'static str,
    message: &'a str,
    help: Option<&'a str>,
    location: Location<'a>,
    related: Vec<Location<'a>>,
}

#[derive(Serialize)]
struct Location<'a> {
    path: String,
    start: Position,
    end: Position,
    message: Option<&'a str>,
}

#[derive(Serialize)]
struct Position {
    line: u32,
    column: u32,
    offset: u32,
}

#[derive(Serialize)]
struct JsonSummary {
    by_severity: BySeverity,
    files_checked: usize,
    objects_checked: usize,
    files_skipped: usize,
}

#[derive(Serialize)]
struct BySeverity {
    error: usize,
    warning: usize,
    info: usize,
}

#[cfg(test)]
mod tests {
    use kubelint_diagnostics::{Diagnostic, Label, Severity};
    use kubelint_source::{FileId, SourceFile, TextRange};

    use super::JsonRenderer;
    use crate::Summary;

    fn range(start: u32, end: u32) -> TextRange {
        TextRange::new(start.into(), end.into())
    }

    #[test]
    fn renders_every_field() {
        let files = [
            SourceFile::new("dir\\a.yaml", "kind: Pod\nname: é x\n"),
            SourceFile::new("b.yaml", "kind: Pod\n"),
        ];
        let diagnostics = [
            Diagnostic::new(
                "KL1004",
                Severity::Warning,
                "duplicate object",
                Label::new(FileId(0), range(16, 19)).with_message("second here"),
            )
            .with_help("rename one of them")
            .with_secondary(Label::new(FileId(1), range(0, 9))),
            Diagnostic::new(
                "KL9999",
                Severity::Info,
                "unnamed rule",
                Label::new(FileId(1), range(10, 10)),
            ),
        ];
        let summary = Summary {
            files_checked: 2,
            objects_checked: 2,
            files_skipped: 1,
            elapsed: Some(std::time::Duration::from_millis(5)),
        };
        let out = JsonRenderer.render(
            &files,
            &diagnostics,
            |code| (code == "KL1004").then_some("duplicate-object"),
            &summary,
        );
        insta::assert_snapshot!(out);
    }

    #[test]
    fn no_findings() {
        let out = JsonRenderer.render(&[], &[], |_| None, &Summary::default());
        insta::assert_snapshot!(out);
    }
}
