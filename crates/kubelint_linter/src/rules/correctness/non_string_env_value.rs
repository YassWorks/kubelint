//! KL1009 `non-string-env-value`.

use kubelint_diagnostics::Severity;
use kubelint_yaml::{Node, NodeKind, ResolvedScalar};

use crate::{Category, ObjectContext, Rule, RuleMeta};

/// Reports a container env var whose `value` YAML reads as a number or a boolean.
///
/// `EnvVar.value` is a string in the Kubernetes API, and the decoder does not convert:
/// `value: 8080` or `value: yes` makes the whole object fail to decode. Null (no value)
/// is an empty string and is fine; mappings and sequences are schema problems.
pub struct NonStringEnvValue;

static META: RuleMeta = RuleMeta {
    code: "KL1009",
    name: "non-string-env-value",
    category: Category::Correctness,
    default_severity: Severity::Error,
    default_enabled: true,
    summary: "An env var value that YAML reads as a number or boolean.",
    explanation: r#"## What it does

Reports a container environment variable whose `value` is not a string once YAML has
read it: an unquoted number such as `8080` or `1.10`, or a boolean such as `true`, `yes`
or `off`.

## Why it matters

Environment variable values are strings in the Kubernetes API. Kubernetes reads YAML
with YAML 1.1 rules, where unquoted `8080` is a number and `yes`, `on` and `off` are
booleans, and it refuses to turn them into strings: the object fails with "cannot
unmarshal number into Go struct field EnvVar.value of type string". Tools that do
convert can change the value on the way, for example `1.10` to `1.1` or `0755` to `493`.

## Example

```yaml
env:
- name: PORT
  value: 8080
- name: DEBUG
  value: yes
```

Use instead:

```yaml
env:
- name: PORT
  value: "8080"
- name: DEBUG
  value: "yes"
```
"#,
};

impl Rule for NonStringEnvValue {
    fn meta(&self) -> &'static RuleMeta {
        &META
    }

    fn check_object(&self, cx: &mut ObjectContext<'_>) {
        let Some(pod) = cx.object().pod_spec() else {
            return;
        };
        for container in pod.containers() {
            let vars = container.env().into_iter().flat_map(Node::items);
            for var in vars {
                let Some(value) = var.get("value") else {
                    continue;
                };
                let NodeKind::Scalar(scalar) = &value.kind else {
                    continue;
                };
                let type_name = match scalar.resolve_kubernetes() {
                    ResolvedScalar::Null | ResolvedScalar::String => continue,
                    ResolvedScalar::Bool(_) => "a boolean",
                    ResolvedScalar::Int(_) | ResolvedScalar::Uint(_) | ResolvedScalar::Float(_) => {
                        "a number"
                    }
                };
                let subject = match var.get("name").and_then(Node::as_str) {
                    Some(name) => format!("env var `{name}`"),
                    None => "env var".to_owned(),
                };
                let message =
                    format!("{subject} has a value that YAML reads as {type_name}, not a string");
                let help = format!("quote the value: `value: \"{}\"`", scalar.value);
                let diagnostic = cx.diagnostic(value.range, message).with_help(help);
                cx.report(diagnostic);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::NonStringEnvValue;

    #[test]
    fn fixture() {
        insta::assert_snapshot!(crate::testing::check_fixture(&NonStringEnvValue));
    }
}
