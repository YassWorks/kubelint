//! KL1001 `missing-api-version`.

use kubelint_diagnostics::Severity;

use super::{TypeField, check_type_field, entry_range, missing_field_range};
use crate::{Category, ObjectContext, Rule, RuleMeta};

/// Reports an object without an `apiVersion`, or with an empty one.
pub struct MissingApiVersion;

static META: RuleMeta = RuleMeta {
    code: "KL1001",
    name: "missing-api-version",
    category: Category::Correctness,
    default_severity: Severity::Error,
    default_enabled: true,
    summary: "Object has no `apiVersion`.",
    explanation: r#"## What it does

Reports a Kubernetes object that has no `apiVersion` field, or whose `apiVersion` is
empty.

## Why it matters

`apiVersion` and `kind` together tell Kubernetes what an object is. Without
`apiVersion`, `kubectl apply` fails with "apiVersion not set" and nothing in the file is
applied.

## Example

```yaml
kind: Deployment
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

`kubectl api-resources` lists the `apiVersion` of every kind your cluster serves.
"#,
};

impl Rule for MissingApiVersion {
    fn meta(&self) -> &'static RuleMeta {
        &META
    }

    fn check_object(&self, cx: &mut ObjectContext<'_>) {
        let object = cx.object();
        let Some(problem) = check_type_field(object, "apiVersion") else {
            return;
        };
        let kind = object.kind().and_then(|kind| kind.as_str());
        let subject = match kind {
            Some(kind) if !kind.trim().is_empty() => format!("`{kind}` object"),
            _ => "object".to_owned(),
        };
        let (range, message) = match problem {
            TypeField::Missing => (
                missing_field_range(object, "kind"),
                format!("{subject} has no `apiVersion`"),
            ),
            TypeField::Empty(entry) => (
                entry_range(entry),
                format!("{subject} has an empty `apiVersion`"),
            ),
        };
        let has_kind = kind.is_some_and(|kind| !kind.trim().is_empty());
        let help = match kind.and_then(kubelint_k8s::stable_api_version) {
            Some(api_version) => format!("add `apiVersion: {api_version}`"),
            None if has_kind => {
                "add the `apiVersion` of this kind; `kubectl api-resources` lists them".to_owned()
            }
            None => "add `apiVersion` and `kind`, such as `apiVersion: apps/v1` and \
                     `kind: Deployment`"
                .to_owned(),
        };
        let diagnostic = cx.diagnostic(range, message).with_help(help);
        cx.report(diagnostic);
    }
}

#[cfg(test)]
mod tests {
    use super::MissingApiVersion;

    #[test]
    fn fixture() {
        insta::assert_snapshot!(crate::testing::check_fixture(&MissingApiVersion));
    }
}
