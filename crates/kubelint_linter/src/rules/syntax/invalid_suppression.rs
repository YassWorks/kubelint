//! KL0003 `invalid-suppression`.

use kubelint_diagnostics::{Diagnostic, Label, Severity};
use kubelint_source::FileId;
use kubelint_yaml::Comment;

use crate::context::LintObject;
use crate::suppression::{Target, ignore_annotation, parse_directive, target};
use crate::{Category, Rule, RuleMeta, suggest_rule};

/// Reports suppression comments and `kubelint/ignore` annotations that silence nothing
/// because a rule they name is unknown, is KL0001, or because they name no rule at all.
///
/// Suppression comments belong to no object, so the engine calls
/// `InvalidSuppression::diagnostics` for every file when this rule is enabled.
pub struct InvalidSuppression;

static META: RuleMeta = RuleMeta {
    code: "KL0003",
    name: "invalid-suppression",
    category: Category::Syntax,
    default_severity: Severity::Warning,
    default_enabled: true,
    summary: "A suppression names a rule that does not exist or cannot be suppressed.",
    explanation: include_str!("invalid_suppression.md"),
};

impl InvalidSuppression {
    pub(crate) fn diagnostics(
        file: FileId,
        comments: &[Comment],
        objects: &[LintObject<'_>],
        severity: Severity,
    ) -> Vec<Diagnostic> {
        let mut out = Vec::new();
        let mut report = |message: String, range, help: String| {
            out.push(
                Diagnostic::new(META.code, severity, message, Label::new(file, range))
                    .with_help(help),
            );
        };
        for comment in comments {
            let Some(directive) = parse_directive(comment) else {
                continue;
            };
            if directive.rules.is_empty() {
                report(
                    "suppression comment names no rule".to_owned(),
                    comment.range,
                    "list the rule codes or names to silence, such as \
                     `# kubelint-disable-line KL1001`"
                        .to_owned(),
                );
            }
            for (selector, range) in &directive.rules {
                if let Some((message, help)) = problem(selector) {
                    report(message, *range, help);
                }
            }
        }
        for (value, selectors) in objects
            .iter()
            .filter_map(|lint_object| ignore_annotation(lint_object.object))
        {
            if selectors.is_empty() {
                report(
                    "`kubelint/ignore` annotation names no rule".to_owned(),
                    value.range,
                    "list the rule codes or names to silence, such as \
                     `kubelint/ignore: \"KL2001,KL3003\"`"
                        .to_owned(),
                );
            }
            for selector in selectors {
                if let Some((message, help)) = problem(selector) {
                    report(message, value.range, help);
                }
            }
        }
        out
    }
}

/// Why `selector` silences nothing, as a message and a help, or `None` when it names a
/// rule that can be suppressed.
fn problem(selector: &str) -> Option<(String, String)> {
    match target(selector) {
        Target::Rule(_) => None,
        Target::Unknown => Some((
            format!("`{selector}` is not a rule code or name"),
            match suggest_rule(selector) {
                Some(suggestion) => format!("did you mean `{suggestion}`?"),
                None => "name a rule by its code (`KL1001`) or its name \
                         (`missing-api-version`); categories and prefixes are not \
                         accepted here"
                    .to_owned(),
            },
        )),
        Target::AlwaysOn => Some((
            format!("`{selector}` cannot be suppressed"),
            "syntax errors are always reported; fix the YAML instead".to_owned(),
        )),
    }
}

impl Rule for InvalidSuppression {
    fn meta(&self) -> &'static RuleMeta {
        &META
    }
}
