//! The rule snapshot test harness.
//!
//! Every rule has a fixture at
//! `crates/kubelint_linter/resources/test/fixtures/<category>/<CODE>.yaml` with cases
//! that must be reported and cases that must not, one YAML document each, each preceded
//! by a comment. A rule's test lints its fixture with only that rule enabled
//! and snapshots the result:
//!
//! ```ignore
//! #[test]
//! fn fixture() {
//!     insta::assert_snapshot!(crate::testing::check_fixture(&MissingKind));
//! }
//! ```
//!
//! The rendering is plain text close to the human output, but owned by this harness, so
//! renderer changes do not churn every rule snapshot. Positions are shown as
//! `line:column` (1-based, column in characters), which keeps snapshots readable and
//! independent of line endings.

use std::fmt::Write as _;
use std::path::PathBuf;

use kubelint_diagnostics::{Diagnostic, Label};
use kubelint_source::{SourceFile, TextRange};

use crate::{EnabledRule, Linter, Rule, RuleMeta};

/// Where the fixture of a rule lives, relative to the crate root.
pub(crate) fn fixture_path(meta: &RuleMeta) -> String {
    format!(
        "resources/test/fixtures/{}/{}.yaml",
        meta.category.as_str(),
        meta.code
    )
}

/// Lints the rule's fixture with only that rule enabled and renders the findings.
///
/// Panics (failing the test) when the fixture is missing.
pub(crate) fn check_fixture(rule: &'static dyn Rule) -> String {
    let relative = fixture_path(rule.meta());
    let path: PathBuf = [env!("CARGO_MANIFEST_DIR"), &relative].iter().collect();
    let text = std::fs::read_to_string(&path)
        .unwrap_or_else(|err| panic!("cannot read fixture {}: {err}", path.display()));
    // The display path uses `/` on every platform so snapshots match on Windows.
    let display = relative.trim_start_matches("resources/test/fixtures/");
    check_source(rule, &SourceFile::new(display, text))
}

/// Lints one file with only `rule` enabled and renders the findings.
pub(crate) fn check_source(rule: &'static dyn Rule, file: &SourceFile) -> String {
    let linter = Linter::new(vec![EnabledRule::new(rule)]);
    let result = linter.lint(std::slice::from_ref(file));
    let mut out = String::new();
    for diagnostic in &result.diagnostics {
        render(&mut out, file, diagnostic);
    }
    let _ = writeln!(
        out,
        "{} finding(s) in {} object(s)",
        result.diagnostics.len(),
        result.objects_checked
    );
    out
}

/// Renders one diagnostic:
///
/// ```text
/// warning[KL9001]: kind Pod
///  --> correctness/KL9001.yaml:2:7
///   |
/// 2 | kind: Pod
///   |       ^^^ label message
///   = help: help text
/// ```
fn render(out: &mut String, file: &SourceFile, diagnostic: &Diagnostic) {
    let _ = writeln!(
        out,
        "{}[{}]: {}",
        diagnostic.severity.as_str(),
        diagnostic.code,
        diagnostic.message
    );
    render_label(out, file, &diagnostic.primary, '^');
    for label in &diagnostic.secondary {
        render_label(out, file, label, '-');
    }
    if let Some(help) = &diagnostic.help {
        let _ = writeln!(out, "  = help: {help}");
    }
    out.push('\n');
}

/// The location line, the source line and a marker under the label's range. A range
/// that spans lines is marked to the end of its first line.
fn render_label(out: &mut String, file: &SourceFile, label: &Label, marker: char) {
    let text = file.text();
    let index = file.line_index();
    let pos = index.line_col(text, label.range.start());
    let line = index
        .line_range(text, pos.line)
        .unwrap_or_else(|| TextRange::empty(label.range.start()));
    let line_text = &text[line];
    let before = text[line.start().into()..label.range.start().into()]
        .chars()
        .count();
    let end = label.range.end().min(line.end()).max(label.range.start());
    let width = text[label.range.start().into()..end.into()]
        .chars()
        .count()
        .max(1);
    let column = before + 1;
    let gutter = " ".repeat(pos.line.to_string().len());
    let _ = writeln!(
        out,
        "{gutter}--> {}:{}:{column}",
        file.path().display(),
        pos.line
    );
    let _ = writeln!(out, "{gutter} |");
    let _ = writeln!(out, "{} | {line_text}", pos.line);
    let marks = marker.to_string().repeat(width);
    let pad = " ".repeat(before);
    match &label.message {
        Some(message) => {
            let _ = writeln!(out, "{gutter} | {pad}{marks} {message}");
        }
        None => {
            let _ = writeln!(out, "{gutter} | {pad}{marks}");
        }
    }
}

#[cfg(test)]
mod tests {
    use kubelint_diagnostics::Severity;
    use kubelint_source::SourceFile;

    use super::{check_fixture, check_source, fixture_path};
    use crate::{Category, ObjectContext, Rule, RuleMeta};

    /// Reports every object at its `kind` value, with a label, a secondary label on
    /// `apiVersion` when present, and help.
    struct KindReporter;

    static META: RuleMeta = RuleMeta {
        code: "KL9001",
        name: "kind-reporter",
        category: Category::Correctness,
        default_severity: Severity::Warning,
        default_enabled: false,
        summary: "test rule",
        explanation: "test rule",
    };

    impl Rule for KindReporter {
        fn meta(&self) -> &'static RuleMeta {
            &META
        }

        fn check_object(&self, cx: &mut ObjectContext<'_>) {
            let object = cx.object();
            let Some(kind) = object.kind() else { return };
            let mut diagnostic = cx
                .diagnostic(kind.range, format!("kind {}", kind.as_str().unwrap_or("?")))
                .with_help("this is only a test");
            diagnostic.primary = diagnostic.primary.with_message("the kind");
            if let Some(api_version) = object.api_version() {
                diagnostic = diagnostic
                    .with_secondary(cx.label(api_version.range).with_message("its apiVersion"));
            }
            cx.report(diagnostic);
        }
    }

    #[test]
    fn fixture_path_uses_category_and_code() {
        assert_eq!(
            fixture_path(&META),
            "resources/test/fixtures/correctness/KL9001.yaml"
        );
    }

    #[test]
    fn checks_the_fixture_file() {
        insta::assert_snapshot!(check_fixture(&KindReporter));
    }

    #[test]
    fn renders_labels_help_and_counts() {
        let file = SourceFile::new(
            "correctness/KL9001.yaml",
            "# Non-ASCII before the finding: é\napiVersion: v1\nkind: Pöd\n---\n# Not an object.\nname: x\n---\n# Flow style, multi-line range.\n{kind: \"Con\n  figMap\"}\n",
        );
        insta::assert_snapshot!(check_source(&KindReporter, &file));
    }

    #[test]
    fn no_findings() {
        let file = SourceFile::new("x.yaml", "name: not-an-object\n");
        insta::assert_snapshot!(check_source(&KindReporter, &file), @"0 finding(s) in 0 object(s)");
    }
}
