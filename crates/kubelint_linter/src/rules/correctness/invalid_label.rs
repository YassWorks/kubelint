//! KL1010 `invalid-label`.

use kubelint_diagnostics::Severity;
use kubelint_k8s::{MAX_LABEL_LENGTH, Object, is_qualified_name, is_valid_label_value};
use kubelint_yaml::{Entry, Node, NodeKind, ResolvedScalar};

use super::shown;
use crate::{Category, ObjectContext, Rule, RuleMeta};

/// Reports a label key or value that breaks Kubernetes' label syntax, and a label value
/// that YAML reads as a number or boolean.
///
/// Checked maps: the object's `metadata.labels`, the pod template's (and a CronJob's
/// job template's) `metadata.labels`, a workload's `spec.selector.matchLabels` (a
/// ReplicationController's plain `spec.selector`) and a Service's `spec.selector`.
/// Selector `matchExpressions` are left alone. A null value is read as an empty
/// string, which is valid.
pub struct InvalidLabel;

static META: RuleMeta = RuleMeta {
    code: "KL1010",
    name: "invalid-label",
    category: Category::Correctness,
    default_severity: Severity::Error,
    default_enabled: true,
    summary: "A label key or value that breaks Kubernetes' label syntax.",
    explanation: r#"## What it does

Reports label keys and values that Kubernetes rejects, in object and pod template
labels and in label selectors. Also reports label values that YAML reads as a number
or boolean, such as `version: 1.0` or `canary: true`.

## Why it matters

Labels are used to select objects, so their syntax is strict. A key is an optional DNS
subdomain prefix and `/`, then a name of at most 63 letters, digits, `-`, `_` and `.`
that starts and ends with a letter or digit (`app.kubernetes.io/name`, `tier`). A value
follows the same rules as the name, or is empty. Spaces, `:` and `@` are not allowed in
either, and values are not free text.

Label values must also be strings. An unquoted `1.0` or `true` is a number or a
boolean in YAML, and the API server refuses the object.

## Example

```yaml
metadata:
  labels:
    owner: team@example.com
    version: 1.0
```

Use instead:

```yaml
metadata:
  labels:
    owner: team-example
    version: "1.0"
  annotations:
    owner: team@example.com
```

Free text belongs in annotations, which have no such limits.
"#,
};

const KEY_HELP: &str = "a label key is an optional DNS prefix and `/`, then up to 63 letters, \
     digits, `-`, `_` and `.`, for example `app.kubernetes.io/name`";

impl Rule for InvalidLabel {
    fn meta(&self) -> &'static RuleMeta {
        &META
    }

    fn check_object(&self, cx: &mut ObjectContext<'_>) {
        for (map, what) in label_maps(cx.object()) {
            for entry in map.entries() {
                check_entry(cx, entry, what);
            }
        }
    }
}

/// The label maps of `object`, each with how messages call it.
fn label_maps(object: Object<'_>) -> Vec<(&Node, &'static str)> {
    let mut maps: Vec<(&Node, &'static str)> = Vec::new();
    maps.extend(object.labels().map(|labels| (labels, "label")));
    let kind = object.kind().and_then(Node::as_str);
    let spec = object.node().get("spec");
    if object.pod_spec().is_some()
        && let Some(spec) = spec
    {
        let templates: &[&[&str]] = match kind {
            Some("CronJob") => &[
                &["jobTemplate", "metadata", "labels"],
                &["jobTemplate", "spec", "template", "metadata", "labels"],
            ],
            Some("Pod") => &[],
            _ => &[&["template", "metadata", "labels"]],
        };
        for path in templates {
            maps.extend(spec.get_path(path).map(|labels| (labels, "label")));
        }
        let selector = match kind {
            Some("ReplicationController") => spec.get("selector"),
            _ => spec.get_path(&["selector", "matchLabels"]),
        };
        maps.extend(selector.map(|labels| (labels, "selector label")));
    } else if kind == Some("Service")
        && object.group_version().is_none_or(|gv| gv.is_core())
        && let Some(selector) = spec.and_then(|spec| spec.get("selector"))
    {
        maps.push((selector, "selector label"));
    }
    maps
}

fn check_entry(cx: &mut ObjectContext<'_>, entry: &Entry, what: &str) {
    // A key that is not a string is a schema problem.
    let Some(key) = entry.key_str() else {
        return;
    };
    if let Err(error) = is_qualified_name(key) {
        let message = format!("{what} key `{}` is invalid: {error}", shown(key));
        let diagnostic = cx.diagnostic(entry.key.range, message).with_help(KEY_HELP);
        cx.report(diagnostic);
    }
    let NodeKind::Scalar(scalar) = &entry.value.kind else {
        return;
    };
    let type_name = match scalar.resolve_kubernetes() {
        ResolvedScalar::Null => return,
        ResolvedScalar::String => None,
        ResolvedScalar::Bool(_) => Some("a boolean"),
        ResolvedScalar::Int(_) | ResolvedScalar::Uint(_) | ResolvedScalar::Float(_) => {
            Some("a number")
        }
    };
    let value = scalar.value.as_str();
    if let Some(type_name) = type_name {
        let message = format!(
            "{what} `{}` has a value that YAML reads as {type_name}, not a string",
            shown(key)
        );
        let diagnostic = cx
            .diagnostic(entry.value.range, message)
            .with_help(format!("quote the value: `{key}: \"{value}\"`"));
        cx.report(diagnostic);
        return;
    }
    if let Err(error) = is_valid_label_value(value) {
        let message = format!(
            "{what} value `{}` of `{}` is invalid: {error}",
            shown(value),
            shown(key)
        );
        let help = match suggestion(value) {
            _ if value.len() > MAX_LABEL_LENGTH => format!(
                "shorten the value to at most {MAX_LABEL_LENGTH} characters, or move it to \
                 an annotation"
            ),
            Some(fixed) => format!(
                "use letters, digits, `-`, `_` and `.`, for example `{fixed}`; free text \
                 belongs in an annotation"
            ),
            None => "use letters, digits, `-`, `_` and `.`; free text belongs in an \
                     annotation"
                .to_owned(),
        };
        let diagnostic = cx.diagnostic(entry.value.range, message).with_help(help);
        cx.report(diagnostic);
    }
}

/// A valid label value close to `value`: runs of other characters become one `-`, the
/// ends are trimmed and it is shortened to 63 characters. `None` for non-ASCII values
/// and when nothing usable is left.
fn suggestion(value: &str) -> Option<String> {
    if !value.is_ascii() {
        return None;
    }
    let allowed = |c: char| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.');
    let mut out = String::new();
    for c in value.chars() {
        if allowed(c) {
            out.push(c);
        } else if !out.ends_with('-') {
            out.push('-');
        }
    }
    let not_alnum = |c: char| !c.is_ascii_alphanumeric();
    let trimmed = out.trim_matches(not_alnum);
    let shortened = trimmed
        .get(..MAX_LABEL_LENGTH.min(trimmed.len()))?
        .trim_end_matches(not_alnum);
    (!shortened.is_empty() && is_valid_label_value(shortened).is_ok()).then(|| shortened.to_owned())
}

#[cfg(test)]
mod tests {
    use super::{InvalidLabel, suggestion};

    #[test]
    fn fixture() {
        insta::assert_snapshot!(crate::testing::check_fixture(&InvalidLabel));
    }

    #[test]
    fn suggestions() {
        let cases = [
            ("hello world", Some("hello-world")),
            ("team@example.com", Some("team-example.com")),
            ("-v1-", Some("v1")),
            ("a: b", Some("a-b")),
            ("ā", None),
            ("@@", None),
        ];
        for (value, expected) in cases {
            assert_eq!(suggestion(value).as_deref(), expected, "{value}");
        }
        assert_eq!(suggestion(&"a".repeat(70)), Some("a".repeat(63)));
    }
}
