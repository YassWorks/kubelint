//! KL1002 `missing-kind`.

use kubelint_diagnostics::Severity;

use super::{TypeField, check_type_field, entry_range, missing_field_range};
use crate::{Category, ObjectContext, Rule, RuleMeta};

/// Reports an object without a `kind`, or with an empty one.
pub struct MissingKind;

static META: RuleMeta = RuleMeta {
    code: "KL1002",
    name: "missing-kind",
    category: Category::Correctness,
    default_severity: Severity::Error,
    default_enabled: true,
    summary: "Object has no `kind`.",
    explanation: r#"## What it does

Reports a Kubernetes object that has no `kind` field, or whose `kind` is empty.

## Why it matters

`apiVersion` and `kind` together tell Kubernetes what an object is. Without `kind`,
`kubectl apply` fails with "kind not set" and nothing in the file is applied.

## Example

```yaml
apiVersion: apps/v1
metadata:
  name: web
```

Use instead:

```yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: web
```
"#,
};

impl Rule for MissingKind {
    fn meta(&self) -> &'static RuleMeta {
        &META
    }

    fn check_object(&self, cx: &mut ObjectContext<'_>) {
        let object = cx.object();
        let Some(problem) = check_type_field(object, "kind") else {
            return;
        };
        let subject = match object.name().and_then(|name| name.as_str()) {
            Some(name) if !name.trim().is_empty() => format!("object `{name}`"),
            _ => "object".to_owned(),
        };
        let (range, message) = match problem {
            TypeField::Missing => (
                missing_field_range(object, "apiVersion"),
                format!("{subject} has no `kind`"),
            ),
            TypeField::Empty(entry) => {
                (entry_range(entry), format!("{subject} has an empty `kind`"))
            }
        };
        let diagnostic = cx
            .diagnostic(range, message)
            .with_help("add the object's type, such as `kind: Deployment`");
        cx.report(diagnostic);
    }
}

#[cfg(test)]
mod tests {
    use super::MissingKind;

    #[test]
    fn fixture() {
        insta::assert_snapshot!(crate::testing::check_fixture(&MissingKind));
    }
}
