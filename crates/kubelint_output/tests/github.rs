use kubelint_diagnostics::{Diagnostic, Label, Severity};
use kubelint_output::{GithubRenderer, Summary};
use kubelint_source::{FileId, SourceFile, TextRange};

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
    insta::assert_snapshot!(out);
}
