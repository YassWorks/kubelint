//! KL2003 `run-as-root`.

use kubelint_diagnostics::Severity;
use kubelint_k8s::SecurityField;
use kubelint_yaml::{Entry, Node, ResolvedScalar, Scalar};

use super::{container_range, container_subject};
use crate::{Category, ObjectContext, Rule, RuleMeta};

/// Reports what the `restricted` checks `runAsNonRoot` and `runAsUser` report, except for
/// pods with `hostUsers: false`.
pub struct RunAsRoot;

static META: RuleMeta = RuleMeta {
    code: "KL2003",
    name: "run-as-root",
    category: Category::Security,
    default_severity: Severity::Warning,
    default_enabled: false,
    summary: "A pod or container does not require a non-root user.",
    explanation: include_str!("run_as_root.md"),
};

const HELP_NON_ROOT: &str = "set `runAsNonRoot: true` in the container or pod `securityContext`";
const HELP_USER: &str = "set a non-zero `runAsUser`, or remove it and set `runAsNonRoot: true`";

/// The entry of `field` in `context`, unless it is absent or null.
fn setting(context: Option<&Node>, field: SecurityField) -> Option<&Entry> {
    context
        .and_then(|context| context.get_entry(field.key()))
        .filter(|entry| !entry.value.is_kubernetes_null())
}

fn is_zero(entry: &Entry) -> bool {
    match entry.value.as_scalar().map(Scalar::resolve_kubernetes) {
        Some(ResolvedScalar::Int(0)) => true,
        Some(ResolvedScalar::Float(value)) => value == 0.0,
        _ => false,
    }
}

impl Rule for RunAsRoot {
    fn meta(&self) -> &'static RuleMeta {
        &META
    }

    fn check_object(&self, cx: &mut ObjectContext<'_>) {
        let Some(pod) = cx.object().pod_spec() else {
            return;
        };
        if pod.has_user_namespace() {
            return;
        }
        let pod_context = pod.security_context();
        let pod_non_root = setting(pod_context, SecurityField::RunAsNonRoot);
        if let Some(user) = setting(pod_context, SecurityField::RunAsUser).filter(|e| is_zero(e)) {
            cx.report(
                cx.diagnostic(
                    user.key.range.cover(user.value.range),
                    "the pod runs as root (`runAsUser: 0`)",
                )
                .with_help(HELP_USER),
            );
        }
        if let Some(non_root) = pod_non_root.filter(|e| e.value.as_bool() == Some(false)) {
            cx.report(
                cx.diagnostic(
                    non_root.key.range.cover(non_root.value.range),
                    "the pod may run as root (`runAsNonRoot: false`)",
                )
                .with_help(HELP_NON_ROOT),
            );
        }
        for container in pod.containers() {
            let context = container.security_context();
            let subject = container_subject(container);
            if let Some(user) = setting(context, SecurityField::RunAsUser).filter(|e| is_zero(e)) {
                cx.report(
                    cx.diagnostic(
                        user.key.range.cover(user.value.range),
                        format!("{subject} runs as root (`runAsUser: 0`)"),
                    )
                    .with_help(HELP_USER),
                );
            }
            match setting(context, SecurityField::RunAsNonRoot) {
                Some(non_root) if non_root.value.as_bool() == Some(false) => cx.report(
                    cx.diagnostic(
                        non_root.key.range.cover(non_root.value.range),
                        format!("{subject} may run as root (`runAsNonRoot: false`)"),
                    )
                    .with_help(HELP_NON_ROOT),
                ),
                Some(_) => {}
                None if pod_non_root.is_some() => {}
                None => cx.report(
                    cx.diagnostic(
                        container_range(container),
                        format!("{subject} does not set `runAsNonRoot: true`"),
                    )
                    .with_help(HELP_NON_ROOT),
                ),
            }
        }
    }
}
