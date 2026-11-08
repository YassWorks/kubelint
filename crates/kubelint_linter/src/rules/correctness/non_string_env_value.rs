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
    explanation: include_str!("non_string_env_value.md"),
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
            for var in container.env().into_iter().flat_map(Node::items) {
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
                cx.report(cx.diagnostic(value.range, message).with_help(help));
            }
        }
    }
}
