use kubelint_linter::{Category, EnabledRule, LintResult, Linter, rules};
use kubelint_source::SourceFile;

fn lint(objects: usize, rules: Vec<EnabledRule>) -> LintResult {
    let files: Vec<SourceFile> = xtask::corpus(objects)
        .into_iter()
        .map(|(name, content)| SourceFile::new(name, content))
        .collect();
    Linter::new(rules).lint(&files)
}

#[test]
fn corpus_has_exactly_the_requested_objects() {
    for objects in [0, 1, 2, 3, 15, 16, 17, 1000] {
        assert_eq!(
            lint(objects, Vec::new()).objects_checked,
            objects,
            "{objects} objects"
        );
    }
}

#[test]
fn corpus_has_no_syntax_or_correctness_findings() {
    let correctness = rules::ALL
        .iter()
        .filter(|rule| {
            matches!(
                rule.meta().category,
                Category::Syntax | Category::Correctness
            )
        })
        .map(|rule| EnabledRule::new(*rule))
        .collect();
    let codes: Vec<&str> = lint(160, correctness)
        .diagnostics
        .iter()
        .map(|diagnostic| diagnostic.code)
        .collect();
    assert!(codes.is_empty(), "{codes:?}");
}
