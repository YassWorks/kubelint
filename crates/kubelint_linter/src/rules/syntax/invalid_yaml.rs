//! KL0001 `invalid-yaml`.

use kubelint_diagnostics::{Diagnostic, Label, Severity};
use kubelint_source::FileId;
use kubelint_yaml::ParseError;

use crate::{Category, Rule, RuleMeta};

/// Reports YAML syntax errors.
///
/// Syntax errors come from parsing, not from looking at objects, so the engine reports
/// them itself, whatever rules are enabled. The rule is registered so that it has a code,
/// a name and an explanation like every other rule.
pub struct InvalidYaml;

static META: RuleMeta = RuleMeta {
    code: "KL0001",
    name: "invalid-yaml",
    category: Category::Syntax,
    default_severity: Severity::Error,
    default_enabled: true,
    summary: "The file is not valid YAML.",
    explanation: include_str!("invalid_yaml.md"),
};

impl InvalidYaml {
    pub(crate) fn diagnostic(file: FileId, error: &ParseError) -> Diagnostic {
        Diagnostic::new(
            META.code,
            META.default_severity,
            format!("invalid YAML: {}", error.message),
            Label::new(file, error.range),
        )
        .with_help("fix the YAML syntax; kubelint skips this document until it parses")
    }
}

impl Rule for InvalidYaml {
    fn meta(&self) -> &'static RuleMeta {
        &META
    }
}
