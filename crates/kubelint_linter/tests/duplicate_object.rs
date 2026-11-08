use kubelint_linter::rules::correctness::DuplicateObject;
use kubelint_linter::{EnabledRule, Linter};
use kubelint_source::{FileId, SourceFile};

const MANIFEST: &str = include_str!("../resources/test/duplicate_object/service.yaml");

#[test]
fn points_back_at_another_file() {
    let files = [
        SourceFile::new("a.yaml", MANIFEST),
        SourceFile::new("b.yaml", format!("# copy\n{MANIFEST}")),
    ];
    let linter = Linter::new(vec![EnabledRule::new(&DuplicateObject)]);
    let result = linter.lint(&files);
    let [diagnostic] = result.diagnostics.as_slice() else {
        panic!("expected one finding: {:?}", result.diagnostics);
    };
    assert_eq!(diagnostic.primary.file, FileId(1));
    assert_eq!(
        diagnostic.message,
        "`Service` `web` is defined more than once"
    );
    let [first] = diagnostic.secondary.as_slice() else {
        panic!("expected one secondary label");
    };
    assert_eq!(first.file, FileId(0));
    assert_eq!(&MANIFEST[first.range], "name: web");
}

#[test]
fn suppression_in_the_copy_silences_it() {
    let files = [
        SourceFile::new("a.yaml", MANIFEST),
        SourceFile::new(
            "b.yaml",
            format!("# kubelint-disable-file duplicate-object\n{MANIFEST}"),
        ),
    ];
    let linter = Linter::new(vec![EnabledRule::new(&DuplicateObject)]);
    assert!(linter.lint(&files).diagnostics.is_empty());
}
