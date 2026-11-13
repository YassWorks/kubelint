//! KL2003 `run-as-root`.

use kubelint_diagnostics::Severity;
use kubelint_k8s::SecurityField;
use kubelint_yaml::{ResolvedScalar, Scalar};

use super::{container_range, container_subject};
use crate::{Category, ObjectContext, Rule, RuleMeta};

/// Reports a container that runs as root (`runAsUser: 0`) or may do so: no non-zero
/// `runAsUser` and no `runAsNonRoot: true`. Container settings override the pod's.
///
/// Without either setting the image's `USER` decides, which kubelint cannot see.
/// Windows pods use `windowsOptions.runAsUserName` instead and are skipped. Values of
/// the wrong type are schema problems.
pub struct RunAsRoot;

static META: RuleMeta = RuleMeta {
    code: "KL2003",
    name: "run-as-root",
    category: Category::Security,
    default_severity: Severity::Warning,
    default_enabled: false,
    summary: "A container may run as root.",
    explanation: include_str!("run_as_root.md"),
};

const HELP: &str = "set `runAsNonRoot: true` and a non-zero `runAsUser` in the container or pod \
                    `securityContext`";

impl Rule for RunAsRoot {
    fn meta(&self) -> &'static RuleMeta {
        &META
    }

    fn check_object(&self, cx: &mut ObjectContext<'_>) {
        let Some(pod) = cx.object().pod_spec().filter(|pod| !pod.is_windows()) else {
            return;
        };
        for container in pod.containers() {
            let subject = || container_subject(container);
            let (range, message) = match container.security_setting(SecurityField::RunAsUser) {
                Some(user) => match user.value.as_scalar().map(Scalar::resolve_kubernetes) {
                    Some(ResolvedScalar::Int(0)) => (
                        user.key.range.cover(user.value.range),
                        format!("{} runs as root (`runAsUser: 0`)", subject()),
                    ),
                    _ => continue,
                },
                None => match container.security_setting(SecurityField::RunAsNonRoot) {
                    Some(non_root) if non_root.value.as_bool() == Some(false) => (
                        non_root.key.range.cover(non_root.value.range),
                        format!("{} may run as root (`runAsNonRoot: false`)", subject()),
                    ),
                    Some(_) => continue,
                    None => (
                        container_range(container),
                        format!("{} may run as root", subject()),
                    ),
                },
            };
            cx.report(cx.diagnostic(range, message).with_help(HELP));
        }
    }
}
