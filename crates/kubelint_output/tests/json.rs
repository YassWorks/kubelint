use kubelint_diagnostics::{Diagnostic, Label, Severity};
use kubelint_output::{JsonRenderer, Summary};
use kubelint_source::{FileId, SourceFile, TextRange};

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
