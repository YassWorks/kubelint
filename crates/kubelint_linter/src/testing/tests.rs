use kubelint_diagnostics::Severity;
use kubelint_source::SourceFile;

use super::{check_fixture, check_source, fixture_path, read};
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
    let text = read("resources/test/testing/labels_help_and_counts.yaml");
    let file = SourceFile::new("correctness/KL9001.yaml", text);
    insta::assert_snapshot!(check_source(&KindReporter, &file));
}

#[test]
fn no_findings() {
    let file = SourceFile::new("x.yaml", "name: not-an-object\n");
    insta::assert_snapshot!(check_source(&KindReporter, &file));
}
