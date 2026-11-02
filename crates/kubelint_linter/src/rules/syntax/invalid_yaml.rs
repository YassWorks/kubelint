//! KL0001 `invalid-yaml`.

use kubelint_diagnostics::{Diagnostic, Label, Severity};
use kubelint_source::FileId;
use kubelint_yaml::ParseError;

use crate::{Category, Rule, RuleMeta};

/// Reports YAML syntax errors.
///
/// Syntax errors come from parsing, not from looking at objects, so the engine reports
/// them itself, whatever rules are enabled. The rule
/// is registered so that it has a code, a name and an explanation like every other rule.
pub struct InvalidYaml;

static META: RuleMeta = RuleMeta {
    code: "KL0001",
    name: "invalid-yaml",
    category: Category::Syntax,
    default_severity: Severity::Error,
    default_enabled: true,
    summary: "The file is not valid YAML.",
    explanation: r"## What it does

Reports a YAML syntax error, such as a bad indentation, an unclosed bracket or quote, or
a tab used for indentation. This rule cannot be disabled.

## Why it matters

Kubernetes cannot read a document with a syntax error, and `kubectl apply` rejects the
whole file. kubelint cannot check the broken document either: it skips it and continues
with the next document after a `---` line.

## Example

```yaml
apiVersion: v1
kind: ConfigMap
data:
  greeting: hello
 farewell: bye
```

`farewell` is indented less than `greeting` but more than `data`, so it belongs to
neither. Use instead:

```yaml
apiVersion: v1
kind: ConfigMap
data:
  greeting: hello
  farewell: bye
```
",
};

impl InvalidYaml {
    /// The diagnostic for a syntax error found in `file`.
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

#[cfg(test)]
mod tests {
    use super::InvalidYaml;

    #[test]
    fn fixture() {
        insta::assert_snapshot!(crate::testing::check_fixture(&InvalidYaml));
    }
}
