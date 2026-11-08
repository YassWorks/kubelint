//! Snapshot tests of the human renderer, without color.

use std::time::Duration;

use kubelint_diagnostics::{Diagnostic, Label, Severity};
use kubelint_output::{HumanRenderer, Summary};
use kubelint_source::{FileId, SourceFile, TextRange, TextSize};

/// The range of the `n`th occurrence (0-based) of `needle` in file `file`.
fn find(files: &[SourceFile], file: u32, needle: &str, n: usize) -> TextRange {
    let text = files[file as usize].text();
    let start = text.match_indices(needle).nth(n).expect("needle in text").0;
    let start = TextSize::try_from(start).expect("small file");
    TextRange::at(start, TextSize::of(needle))
}

fn rule_name(code: &str) -> Option<&'static str> {
    match code {
        "KL1001" => Some("missing-api-version"),
        "KL1004" => Some("duplicate-object"),
        "KL3001" => Some("missing-memory-limit"),
        _ => None,
    }
}

fn render(files: &[SourceFile], diagnostics: &[Diagnostic], summary: Summary) -> String {
    HumanRenderer::new(false).render(files, diagnostics, rule_name, &summary)
}

fn files() -> Vec<SourceFile> {
    vec![
        SourceFile::new("deploy/web.yaml", include_str!("fixtures/web.yaml")),
        SourceFile::new(
            "deploy/copy.yaml",
            "kind: Deployment\r\nmetadata:\r\n  name: web\r\n",
        ),
    ]
}

#[test]
fn error_with_label_help_and_rule_name() {
    let files = files();
    let diagnostics = [Diagnostic::new(
        "KL1001",
        Severity::Error,
        "`Deployment` object has no `apiVersion`",
        Label::new(FileId(1), find(&files, 1, "kind: Deployment", 0)).with_message("this object"),
    )
    .with_help("add `apiVersion: apps/v1`")];
    let summary = Summary {
        files_checked: 2,
        objects_checked: 2,
        files_skipped: 0,
        elapsed: None,
    };
    insta::assert_snapshot!(render(&files, &diagnostics, summary));
}

#[test]
fn secondary_labels_in_the_same_and_another_file() {
    let files = files();
    let diagnostics = [
        Diagnostic::new(
            "KL1004",
            Severity::Error,
            "`Deployment` `web` is defined twice",
            Label::new(FileId(1), find(&files, 1, "web", 0)).with_message("defined again here"),
        )
        .with_secondary(
            Label::new(FileId(0), find(&files, 0, "web", 0)).with_message("first defined here"),
        ),
        Diagnostic::new(
            "KL3001",
            Severity::Warning,
            "container `web` has no memory limit",
            Label::new(FileId(0), find(&files, 0, "- name: web", 0)),
        )
        .with_secondary(
            Label::new(FileId(0), find(&files, 0, "kind: Deployment", 0))
                .with_message("in this Deployment"),
        ),
    ];
    let summary = Summary {
        files_checked: 2,
        objects_checked: 2,
        files_skipped: 0,
        elapsed: None,
    };
    insta::assert_snapshot!(render(&files, &diagnostics, summary));
}

#[test]
fn empty_and_multi_line_ranges_and_unknown_rule_name() {
    let files = files();
    let start = find(&files, 0, "spec:\n  template", 0).start();
    let end = find(&files, 0, "nginx:1.27", 0).end();
    let diagnostics = [
        Diagnostic::new(
            "KL0001",
            Severity::Error,
            "invalid YAML: did not find expected key",
            Label::new(
                FileId(0),
                TextRange::empty(find(&files, 0, "café", 0).end()),
            ),
        ),
        Diagnostic::new(
            "KL9999",
            Severity::Info,
            "a range over several lines",
            Label::new(FileId(0), TextRange::new(start, end)).with_message("this spec"),
        ),
    ];
    let summary = Summary {
        files_checked: 1,
        objects_checked: 0,
        files_skipped: 0,
        elapsed: None,
    };
    insta::assert_snapshot!(render(&files, &diagnostics, summary));
}

#[test]
fn summary_lines() {
    let files = files();
    let label = Label::new(FileId(0), find(&files, 0, "web", 0));
    let diagnostic = |severity| Diagnostic::new("KL9999", severity, "x", label.clone());
    let summary = |elapsed| Summary {
        files_checked: 3,
        objects_checked: 1,
        files_skipped: 0,
        elapsed,
    };
    let last_line = |diagnostics: &[Diagnostic], elapsed| {
        render(&files, diagnostics, summary(elapsed))
            .lines()
            .last()
            .unwrap_or_default()
            .to_owned()
    };
    assert_eq!(
        last_line(&[], Some(Duration::from_millis(42))),
        "All checks passed. Checked 3 files and 1 object in 42ms."
    );
    assert_eq!(
        last_line(
            &[diagnostic(Severity::Warning), diagnostic(Severity::Warning)],
            Some(Duration::from_millis(12_340))
        ),
        "Found 2 warnings. Checked 3 files and 1 object in 12.3s."
    );
    assert_eq!(
        last_line(
            &[
                diagnostic(Severity::Info),
                diagnostic(Severity::Error),
                diagnostic(Severity::Warning),
                diagnostic(Severity::Info),
            ],
            None
        ),
        "Found 1 error, 1 warning and 2 info. Checked 3 files and 1 object."
    );
    let skipped = |files_skipped| Summary {
        files_checked: 1,
        objects_checked: 2,
        files_skipped,
        elapsed: Some(Duration::from_millis(5)),
    };
    let last_line = |summary| {
        render(&files, &[], summary)
            .lines()
            .last()
            .unwrap_or_default()
            .to_owned()
    };
    assert_eq!(
        last_line(skipped(1)),
        "All checks passed. Checked 1 file and 2 objects, skipped 1 file in 5ms."
    );
    assert_eq!(
        last_line(skipped(4)),
        "All checks passed. Checked 1 file and 2 objects, skipped 4 files in 5ms."
    );
}

#[test]
fn color_adds_ansi_codes_only_when_asked() {
    let files = files();
    let diagnostics = [Diagnostic::new(
        "KL1001",
        Severity::Error,
        "message",
        Label::new(FileId(0), find(&files, 0, "web", 0)),
    )];
    let summary = Summary::default();
    let colored = HumanRenderer::new(true).render(&files, &diagnostics, rule_name, &summary);
    assert!(colored.contains("\u{1b}["));
    assert!(!render(&files, &diagnostics, summary).contains('\u{1b}'));
}

#[test]
fn a_label_in_an_unknown_file_does_not_panic() {
    let files = files();
    let diagnostics = [Diagnostic::new(
        "KL1001",
        Severity::Error,
        "message",
        Label::new(FileId(7), TextRange::empty(0.into())),
    )];
    let out = render(&files, &diagnostics, Summary::default());
    assert!(out.starts_with("error[KL1001]: message"));
}
