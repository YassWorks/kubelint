use kubelint_diagnostics::{Label, Severity};
use kubelint_source::FileId;

use super::*;
use crate::testing::read;

fn directive(text: &str) -> Option<Directive> {
    let parse = kubelint_yaml::parse(text);
    parse.comments.first().and_then(parse_directive)
}

#[test]
fn parses_rules_reason_and_positions() {
    let text = "a: 1  # kubelint-disable-line KL1001,  missing-kind -- generated é\n";
    let directive = directive(text).expect("a directive");
    assert_eq!(directive.kind, DirectiveKind::Line);
    let rules: Vec<_> = directive
        .rules
        .iter()
        .map(|(rule, r)| {
            let at = &text[usize::from(r.start())..usize::from(r.end())];
            (rule.as_str(), at)
        })
        .collect();
    assert_eq!(
        rules,
        [("KL1001", "KL1001"), ("missing-kind", "missing-kind")]
    );
    assert_eq!(directive.reason.as_deref(), Some("generated é"));
}

#[test]
fn ordinary_comments_are_not_directives() {
    assert_eq!(directive("# just a note\n"), None);
    assert_eq!(directive("# see kubelint-disable-line KL1001\n"), None);
    assert_eq!(directive("# kubelint-disable-lines KL1001\n"), None);
    let bare = directive("#kubelint-disable-file\n").expect("a directive");
    assert_eq!((bare.kind, bare.rules.len()), (DirectiveKind::File, 0));
}

fn diagnostic(code: &'static str, file: &SourceFile, line: u32) -> Diagnostic {
    let start = file.line_index().offset(line, 1).expect("the line exists");
    Diagnostic::new(
        code,
        Severity::Error,
        "x",
        Label::new(FileId(0), TextRange::empty(start)),
    )
}

#[test]
fn next_line_skips_comment_lines() {
    let file = SourceFile::new("f.yaml", read("resources/test/suppression/next_line.yaml"));
    let suppressions = Suppressions::new(&file, &kubelint_yaml::parse(file.text()).comments);
    let suppressed = |code, line| suppressions.suppresses(&file, &diagnostic(code, &file, line));
    assert!(suppressed("KL1001", 3));
    assert!(!suppressed("KL1002", 3));
    assert!(suppressed("KL1002", 4));
    assert!(!suppressed("KL1001", 4));
    assert!(!suppressed("KL1001", 5));
}

#[test]
fn file_directives_cover_every_line_but_not_invalid_yaml() {
    let text = "a: 1\n# kubelint-disable-file KL1001, invalid-yaml, KL0001, nope\n";
    let file = SourceFile::new("f.yaml", text);
    let suppressions = Suppressions::new(&file, &kubelint_yaml::parse(text).comments);
    assert!(suppressions.suppresses(&file, &diagnostic("KL1001", &file, 1)));
    assert!(!suppressions.suppresses(&file, &diagnostic("KL0001", &file, 1)));
}
