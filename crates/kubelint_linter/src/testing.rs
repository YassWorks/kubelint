//! The rule snapshot test harness.
//!
//! Every rule has a fixture at
//! `crates/kubelint_linter/resources/test/fixtures/<category>/<CODE>.yaml` with cases
//! that must be reported and cases that must not, one YAML document each, each preceded
//! by a comment. `rules/tests.rs` lints each fixture with only its rule enabled and
//! snapshots the result.
//!
//! The rendering is plain text close to the human output, but owned by this harness, so
//! renderer changes do not churn every rule snapshot. Positions are shown as
//! `line:column` (1-based, column in characters), which keeps snapshots readable and
//! independent of line endings.

use std::fmt::{self, Write as _};
use std::path::PathBuf;

use kubelint_diagnostics::{Diagnostic, Label};
use kubelint_source::{SourceFile, TextRange};

use crate::{EnabledRule, LintResult, Linter, Rule, RuleMeta};

/// Where the fixture of a rule lives, relative to the crate root.
pub(crate) fn fixture_path(meta: &RuleMeta) -> String {
    format!(
        "resources/test/fixtures/{}/{}.yaml",
        meta.category.as_str(),
        meta.code
    )
}

/// Lints the rule's fixture with only that rule enabled and renders the findings.
pub(crate) fn check_fixture(rule: &'static dyn Rule) -> String {
    let relative = fixture_path(rule.meta());
    // The display path uses `/` on every platform so snapshots match on Windows.
    let display = relative.trim_start_matches("resources/test/fixtures/");
    check_source(rule, &SourceFile::new(display, read(&relative)))
}

/// Reads a test file given relative to the crate root. Panics when it is missing.
pub(crate) fn read(relative: &str) -> String {
    let path: PathBuf = [env!("CARGO_MANIFEST_DIR"), relative].iter().collect();
    std::fs::read_to_string(&path)
        .unwrap_or_else(|err| panic!("cannot read {}: {err}", path.display()))
}

/// Lints one file with only `rule` enabled and renders the findings.
pub(crate) fn check_source(rule: &'static dyn Rule, file: &SourceFile) -> String {
    let result = Linter::new(vec![EnabledRule::new(rule)]).lint(std::slice::from_ref(file));
    let mut out = String::new();
    render(&mut out, file, &result).expect("writing to a `String` cannot fail");
    out
}

fn render(out: &mut String, file: &SourceFile, result: &LintResult) -> fmt::Result {
    for diagnostic in &result.diagnostics {
        render_diagnostic(out, file, diagnostic)?;
    }
    writeln!(
        out,
        "{} finding(s) in {} object(s)",
        result.diagnostics.len(),
        result.objects_checked
    )
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
fn render_diagnostic(out: &mut String, file: &SourceFile, diagnostic: &Diagnostic) -> fmt::Result {
    writeln!(
        out,
        "{}[{}]: {}",
        diagnostic.severity.as_str(),
        diagnostic.code,
        diagnostic.message
    )?;
    render_label(out, file, &diagnostic.primary, '^')?;
    for label in &diagnostic.secondary {
        render_label(out, file, label, '-')?;
    }
    if let Some(help) = &diagnostic.help {
        writeln!(out, "  = help: {help}")?;
    }
    writeln!(out)
}

/// The location line, the source line and a marker under the label's range. A range
/// that spans lines is marked to the end of its first line.
fn render_label(out: &mut String, file: &SourceFile, label: &Label, marker: char) -> fmt::Result {
    let text = file.text();
    let index = file.line_index();
    let start = label.range.start();
    let pos = index.line_col(text, start);
    let line = index
        .line_range(text, pos.line)
        .unwrap_or_else(|| TextRange::empty(start));
    let before = text[line.start().into()..start.into()].chars().count();
    let end = label.range.end().min(line.end()).max(start);
    let width = text[start.into()..end.into()].chars().count().max(1);
    let gutter = " ".repeat(pos.line.to_string().len());
    writeln!(
        out,
        "{gutter}--> {}:{}:{}",
        file.path().display(),
        pos.line,
        before + 1
    )?;
    writeln!(out, "{gutter} |")?;
    writeln!(out, "{} | {}", pos.line, &text[line])?;
    let marks = marker.to_string().repeat(width);
    write!(out, "{gutter} | {}{marks}", " ".repeat(before))?;
    match &label.message {
        Some(message) => writeln!(out, " {message}"),
        None => writeln!(out),
    }
}

mod tests;
