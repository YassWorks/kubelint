//! KL2008 `secret-in-env-literal`.

use kubelint_diagnostics::Severity;
use kubelint_yaml::{Node, ResolvedScalar};

use super::container_subject;
use crate::{Category, ObjectContext, Rule, RuleMeta};

/// Reports an env var whose name ends in a secret word and whose `value` is a literal.
pub struct SecretInEnvLiteral;

static META: RuleMeta = RuleMeta {
    code: "KL2008",
    name: "secret-in-env-literal",
    category: Category::Security,
    default_severity: Severity::Warning,
    default_enabled: true,
    summary: "An env var that looks like a secret has its value written in the manifest.",
    explanation: include_str!("secret_in_env_literal.md"),
};

/// Name endings that mark a secret, compared upper case with `_`, `-` and `.` removed.
const SECRET_SUFFIXES: [&str; 9] = [
    "PASSWORD",
    "PASSWD",
    "SECRET",
    "TOKEN",
    "APIKEY",
    "SECRETKEY",
    "PRIVATEKEY",
    "ACCESSKEY",
    "CREDENTIALS",
];

fn is_secret_name(name: &str) -> bool {
    let name = name.to_ascii_uppercase().replace(['_', '-', '.'], "");
    SECRET_SUFFIXES.iter().any(|suffix| name.ends_with(suffix))
}

/// A single `$(VAR)`, which Kubernetes expands from another env var, or `${VAR}`, a
/// templating placeholder (envsubst) whose final value kubelint cannot see.
/// <https://kubernetes.io/docs/tasks/inject-data-application/define-interdependent-environment-variables/>
fn is_reference(value: &str) -> bool {
    let inner = value
        .strip_prefix("$(")
        .and_then(|rest| rest.strip_suffix(')'))
        .or_else(|| {
            value
                .strip_prefix("${")
                .and_then(|rest| rest.strip_suffix('}'))
        });
    inner.is_some_and(|inner| !inner.contains(['$', '(', ')', '{', '}']))
}

impl Rule for SecretInEnvLiteral {
    fn meta(&self) -> &'static RuleMeta {
        &META
    }

    fn check_object(&self, cx: &mut ObjectContext<'_>) {
        let Some(pod) = cx.object().pod_spec() else {
            return;
        };
        for container in pod.containers() {
            for var in container.env().into_iter().flat_map(Node::items) {
                let Some(name) = var
                    .get("name")
                    .and_then(Node::as_str)
                    .filter(|name| is_secret_name(name))
                else {
                    continue;
                };
                let Some(entry) = var.get_entry("value").filter(|entry| {
                    entry
                        .value
                        .as_scalar()
                        .filter(|scalar| scalar.resolve_kubernetes() == ResolvedScalar::String)
                        .map(|scalar| scalar.value.trim())
                        .is_some_and(|value| !value.is_empty() && !is_reference(value))
                }) else {
                    continue;
                };
                let message = format!(
                    "env var `{name}` of {} has a literal value",
                    container_subject(container)
                );
                cx.report(
                    cx.diagnostic(entry.key.range.cover(entry.value.range), message)
                        .with_help(
                            "store the value in a Secret and read it with \
                             `valueFrom.secretKeyRef`",
                        ),
                );
            }
        }
    }
}
