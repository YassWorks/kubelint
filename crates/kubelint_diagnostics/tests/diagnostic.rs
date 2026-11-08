use kubelint_diagnostics::{Applicability, Diagnostic, Edit, Fix, Label, Severity};
use kubelint_source::{FileId, TextRange};

fn range(a: u32, b: u32) -> TextRange {
    TextRange::new(a.into(), b.into())
}

#[test]
fn severities_are_ordered_for_fail_on() {
    assert!(Severity::Info < Severity::Warning);
    assert!(Severity::Warning < Severity::Error);
    assert_eq!(Severity::Info.as_str(), "info");
    assert_eq!(Severity::Error.as_str(), "error");
}

#[test]
fn builder_collects_everything() {
    let fix = Fix {
        applicability: Applicability::Safe,
        edits: vec![Edit {
            range: range(5, 9),
            replacement: "false".to_owned(),
        }],
    };
    let d = Diagnostic::new(
        "KL1004",
        Severity::Warning,
        "duplicate object",
        Label::new(FileId(1), range(0, 4)),
    )
    .with_secondary(Label::new(FileId(0), range(20, 24)).with_message("first defined here"))
    .with_help("rename one of them")
    .with_fix(fix.clone());

    assert_eq!(d.primary.message, None);
    assert_eq!(d.secondary.len(), 1);
    assert_eq!(d.secondary[0].file, FileId(0));
    assert_eq!(
        d.secondary[0].message.as_deref(),
        Some("first defined here")
    );
    assert_eq!(d.help.as_deref(), Some("rename one of them"));
    assert_eq!(d.fix, Some(fix));
}
