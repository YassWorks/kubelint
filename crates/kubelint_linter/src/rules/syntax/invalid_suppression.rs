//! KL0003 `invalid-suppression`.

use kubelint_diagnostics::{Diagnostic, Label, Severity};
use kubelint_source::FileId;
use kubelint_yaml::Comment;

use crate::suppression::{Target, parse_directive, target};
use crate::{Category, Rule, RuleMeta};

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
    explanation: r"## What it does

Checks `# kubelint-disable-next-line`, `# kubelint-disable-line` and
`# kubelint-disable-file` comments. It reports a rule code or name that no rule has, a
comment that names `KL0001` `invalid-yaml` (syntax errors cannot be suppressed), and a
comment that names no rule at all.

## Why it matters

A suppression with a typo silences nothing, so the finding it was meant to hide is still
reported, or, worse, the comment looks like it documents an accepted risk that nobody
actually reviewed. Reporting it keeps suppressions honest.

## Example

```yaml
securityContext:
  # kubelint-disable-next-line privileged-containers
  privileged: true
```

The rule is called `privileged-container`. Use instead:

```yaml
securityContext:
  # kubelint-disable-next-line privileged-container -- the CNI plugin needs this
  privileged: true
```
",
};

impl InvalidSuppression {
    /// The findings for the suppression comments of one file, at `severity`.
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
                        "name a rule by its code (`KL1001`) or its name \
                         (`missing-api-version`); categories and prefixes are not \
                         accepted here",
                    ),
                    Target::AlwaysOn => (
                        format!("`{selector}` cannot be suppressed"),
                        "syntax errors are always reported; fix the YAML instead",
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

#[cfg(test)]
mod tests {
    use super::InvalidSuppression;

    #[test]
    fn fixture() {
        insta::assert_snapshot!(crate::testing::check_fixture(&InvalidSuppression));
    }
}
