//! GitHub Actions workflow commands, documented in `docs/user/output-formats.md`.

use kubelint_diagnostics::{Diagnostic, Severity};
use kubelint_source::SourceFile;

use crate::human::summary_line;
use crate::{Summary, file, message_with_help, path_string};

/// Renders diagnostics as GitHub Actions workflow commands, one line each, then the
/// summary line (without timing) as plain log text.
#[derive(Debug, Clone, Copy, Default)]
pub struct GithubRenderer;

impl GithubRenderer {
    /// Renders every diagnostic as an `::error`, `::warning` or `::notice` command.
    ///
    /// ```
    /// use kubelint_diagnostics::{Diagnostic, Label, Severity};
    /// use kubelint_output::{GithubRenderer, Summary};
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
    /// let out = GithubRenderer.render(&files, &diagnostics, |_| Some("missing-api-version"), &summary);
    /// assert!(out.starts_with(
    ///     "::error file=pod.yaml,line=1,col=1,endLine=1,endColumn=10,\
    ///      title=KL1001 missing-api-version::`Pod` object has no `apiVersion`\n"
    /// ));
    /// ```
    pub fn render<'n>(
        &self,
        files: &[SourceFile],
        diagnostics: &[Diagnostic],
        rule_name: impl Fn(&str) -> Option<&'n str>,
        summary: &Summary,
    ) -> String {
        let mut out: String = diagnostics
            .iter()
            .map(|d| command(files, d, rule_name(d.code)))
            .collect();
        let summary = Summary {
            elapsed: None,
            ..*summary
        };
        out.push_str(&summary_line(diagnostics, &summary));
        out.push('\n');
        out
    }
}

/// One workflow command, ending with a newline.
fn command(files: &[SourceFile], diagnostic: &Diagnostic, rule_name: Option<&str>) -> String {
    let level = match diagnostic.severity {
        Severity::Error => "error",
        Severity::Warning => "warning",
        Severity::Info => "notice",
    };
    let label = &diagnostic.primary;
    let location = file(files, label.file).map_or_else(String::new, |file| {
        let index = file.line_index();
        let start = index.line_col(file.text(), label.range.start());
        let end = index.line_col(file.text(), label.range.end());
        format!(
            "file={},line={},col={},endLine={},endColumn={},",
            escape_property(&path_string(file)),
            start.line,
            start.utf16_col,
            end.line,
            end.utf16_col,
        )
    });
    let title = match rule_name {
        Some(name) => format!("{} {name}", diagnostic.code),
        None => diagnostic.code.to_owned(),
    };
    format!(
        "::{level} {location}title={}::{}\n",
        escape_property(&title),
        escape_data(&message_with_help(diagnostic)),
    )
}

/// Escapes a command's message, as the GitHub Actions runner expects.
fn escape_data(text: &str) -> String {
    text.replace('%', "%25")
        .replace('\r', "%0D")
        .replace('\n', "%0A")
}

/// Escapes a property value (`file=`, `title=`), which also cannot hold `:` or `,`.
fn escape_property(text: &str) -> String {
    escape_data(text).replace(':', "%3A").replace(',', "%2C")
}

#[cfg(test)]
mod tests;
