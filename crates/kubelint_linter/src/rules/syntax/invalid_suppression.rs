//! KL0003 `invalid-suppression`.

use kubelint_diagnostics::{Diagnostic, Label, Severity};
use kubelint_source::FileId;
use kubelint_yaml::Comment;

use crate::suppression::{Target, parse_directive, target};
use crate::{Category, Rule, RuleMeta, suggest_rule};

/// Reports suppression comments that silence nothing because a rule they name is
/// unknown, is KL0001, or because they name no rule at all.
///
/// Suppression comments are read by the engine, not from objects, so the engine calls
/// `InvalidSuppression::diagnostics` for every file when this rule is enabled.
pub struct InvalidSuppression;

static META: RuleMeta = RuleMeta {
    code: "KL0003",
    name: "invalid-suppression",
    category: Category::Syntax,
    default_severity: Severity::Warning,
    default_enabled: true,
    summary: "A suppression comment names a rule that does not exist or cannot be suppressed.",
    explanation: include_str!("invalid_suppression.md"),
};

impl InvalidSuppression {
    pub(crate) fn diagnostics(
        file: FileId,
        comments: &[Comment],
        severity: Severity,
    ) -> Vec<Diagnostic> {
        let mut out = Vec::new();
        for comment in comments {
            let Some(directive) = parse_directive(comment) else {
                continue;
            };
            if directive.rules.is_empty() {
                out.push(
                    Diagnostic::new(
                        META.code,
                        severity,
                        "suppression comment names no rule",
                        Label::new(file, comment.range),
                    )
                    .with_help(
                        "list the rule codes or names to silence, such as \
                         `# kubelint-disable-line KL1001`",
                    ),
                );
            }
            for (selector, range) in &directive.rules {
                let (message, help) = match target(selector) {
                    Target::Rule(_) => continue,
                    Target::Unknown => (
                        format!("`{selector}` is not a rule code or name"),
                        match suggest_rule(selector) {
                            Some(suggestion) => format!("did you mean `{suggestion}`?"),
                            None => "name a rule by its code (`KL1001`) or its name \
                                     (`missing-api-version`); categories and prefixes are \
                                     not accepted here"
                                .to_owned(),
                        },
                    ),
                    Target::AlwaysOn => (
                        format!("`{selector}` cannot be suppressed"),
                        "syntax errors are always reported; fix the YAML instead".to_owned(),
                    ),
                };
                out.push(
                    Diagnostic::new(META.code, severity, message, Label::new(file, *range))
                        .with_help(help),
                );
            }
        }
        out
    }
}

impl Rule for InvalidSuppression {
    fn meta(&self) -> &'static RuleMeta {
        &META
    }
}
