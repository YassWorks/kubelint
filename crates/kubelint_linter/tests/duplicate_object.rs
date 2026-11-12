use kubelint_linter::rules::correctness::DuplicateObject;
use kubelint_linter::{EnabledRule, Linter};
use kubelint_source::{FileId, SourceFile};

const MANIFEST: &str = include_str!("../resources/test/duplicate_object/service.yaml");

fn lint(files: &[SourceFile]) -> Vec<kubelint_diagnostics::Diagnostic> {
    Linter::new(vec![EnabledRule::new(&DuplicateObject)])
        .lint(files)
        .diagnostics
}

#[test]
fn copies_in_other_files_are_not_reported() {
    let files = [
        SourceFile::new("base/web.yaml", MANIFEST),
        SourceFile::new("overlays/prod/web.yaml", MANIFEST),
        SourceFile::new("base/web-copy.yaml", MANIFEST),
    ];
    assert!(lint(&files).is_empty());
}

#[test]
fn points_back_into_its_own_file() {
    let files = [
        SourceFile::new("a.yaml", MANIFEST),
        SourceFile::new("b.yaml", format!("{MANIFEST}---\n{MANIFEST}")),
    ];
    let diagnostics = lint(&files);
    let [diagnostic] = diagnostics.as_slice() else {
        panic!("expected one finding: {diagnostics:?}");
    };
    assert_eq!(diagnostic.primary.file, FileId(1));
    let [first] = diagnostic.secondary.as_slice() else {
        panic!("expected one secondary label");
    };
    assert_eq!(first.file, FileId(1));
    assert!(first.range.start() < diagnostic.primary.range.start());
}
