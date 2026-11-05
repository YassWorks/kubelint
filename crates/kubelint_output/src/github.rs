//! The GitHub renderer: workflow commands that GitHub Actions turns into annotations.
//!
//! Format: `::error file=a.yaml,line=1,col=1,endLine=1,endColumn=14,title=...::message`.
//! See <https://docs.github.com/actions/reference/workflow-commands-for-github-actions>.

use std::fmt::Write as _;

use kubelint_diagnostics::{Diagnostic, Severity};
use kubelint_source::SourceFile;

use crate::Summary;
use crate::human::{path_string, summary_line};

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
        let mut out = String::new();
        for diagnostic in diagnostics {
            let command = match diagnostic.severity {
                Severity::Error => "error",
                Severity::Warning => "warning",
                Severity::Info => "notice",
            };
            let _ = write!(out, "::{command} ");
            let label = &diagnostic.primary;
            if let Some(file) = files.get(label.file.0 as usize) {
                let index = file.line_index();
                let start = index.line_col(file.text(), label.range.start());
                let end = index.line_col(file.text(), label.range.end());
                let _ = write!(
                    out,
                    "file={},line={},col={},endLine={},endColumn={},",
                    escape_property(&path_string(file)),
                    start.line,
                    start.utf16_col,
                    end.line,
                    end.utf16_col,
                );
            }
            let title = match rule_name(diagnostic.code) {
                Some(name) => format!("{} {name}", diagnostic.code),
                None => diagnostic.code.to_owned(),
            };
            let message = match &diagnostic.help {
                Some(help) => format!("{}\nhelp: {help}", diagnostic.message),
                None => diagnostic.message.clone(),
            };
            let _ = writeln!(
                out,
                "title={}::{}",
                escape_property(&title),
                escape_data(&message)
            );
        }
        let summary = Summary {
            elapsed: None,
            ..*summary
        };
        out.push_str(&summary_line(diagnostics, &summary));
        out.push('\n');
        out
    }
}

/// Escapes a command's message, as the GitHub Actions runner expects.
fn escape_data(text: &str) -> String {
    text.replace('%', "%25")
        .replace('\r', "%0D")
        .replace('\n', "%0A")
}

/// Escapes a property value (`file=`, `title=`), which additionally cannot hold `:` or
/// `,`.
fn escape_property(text: &str) -> String {
    escape_data(text).replace(':', "%3A").replace(',', "%2C")
}

#[cfg(test)]
mod tests {
    use kubelint_diagnostics::{Diagnostic, Label, Severity};
    use kubelint_source::{FileId, SourceFile, TextRange};

    use super::{GithubRenderer, escape_data, escape_property};
    use crate::Summary;

    #[test]
    fn escaping() {
        assert_eq!(
            escape_data("50% done\r\nnext: a, b"),
            "50%25 done%0D%0Anext: a, b"
        );
        assert_eq!(escape_property("a:b,c%\n"), "a%3Ab%2Cc%25%0A");
    }

    #[test]
    fn renders_levels_positions_and_summary() {
        let files = [
            SourceFile::new("dir\\a,b.yaml", "kind: Pod\nname: é😀x\n"),
            SourceFile::new("b.yaml", "kind: Pod\n"),
        ];
        let diagnostics = [
            Diagnostic::new(
                "KL1004",
                Severity::Warning,
                "duplicate object: 100%",
                Label::new(FileId(0), TextRange::new(22.into(), 23.into())),
            )
            .with_help("rename one of them"),
            Diagnostic::new(
                "KL3005",
                Severity::Info,
                "one replica",
                Label::new(FileId(1), TextRange::new(0.into(), 10.into())),
            ),
            Diagnostic::new(
                "KL9999",
                Severity::Error,
                "in a file that is not part of the run",
                Label::new(FileId(7), TextRange::new(0.into(), 1.into())),
            ),
        ];
        let summary = Summary {
            files_checked: 2,
            objects_checked: 2,
            files_skipped: 0,
            elapsed: Some(std::time::Duration::from_millis(3)),
        };
        let out = GithubRenderer.render(
            &files,
            &diagnostics,
            |code| (code == "KL1004").then_some("duplicate-object"),
            &summary,
        );
        insta::assert_snapshot!(out, @r"
        ::warning file=dir/a%2Cb.yaml,line=2,col=10,endLine=2,endColumn=11,title=KL1004 duplicate-object::duplicate object: 100%25%0Ahelp: rename one of them
        ::notice file=b.yaml,line=1,col=1,endLine=2,endColumn=1,title=KL3005::one replica
        ::error title=KL9999::in a file that is not part of the run
        Found 1 error, 1 warning and 1 info. Checked 2 files and 2 objects.
        ");
    }
}
