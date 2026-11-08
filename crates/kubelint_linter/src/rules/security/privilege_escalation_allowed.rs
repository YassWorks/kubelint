//! KL2002 `privilege-escalation-allowed`.

use kubelint_diagnostics::Severity;
use kubelint_yaml::Node;

use super::{container_range, container_subject};
use crate::{Category, ObjectContext, Rule, RuleMeta};

/// Reports a container that does not set `securityContext.allowPrivilegeEscalation` to
/// false. Kubernetes defaults it to true, and there is no pod-level setting.
///
/// The API server rejects `false` together with `privileged: true` or an added
/// `CAP_SYS_ADMIN`, and on Windows pods (`ValidateSecurityContext` and
/// `validateWindows` in `pkg/apis/core/validation/validation.go`), so those containers
/// are skipped. KL2001 reports privileged ones. A value that is neither a boolean nor
/// null is a schema problem.
pub struct PrivilegeEscalationAllowed;

static META: RuleMeta = RuleMeta {
    code: "KL2002",
    name: "privilege-escalation-allowed",
    category: Category::Security,
    default_severity: Severity::Warning,
    default_enabled: true,
    summary: "A container does not set `allowPrivilegeEscalation: false`.",
    explanation: include_str!("privilege_escalation_allowed.md"),
};

impl Rule for PrivilegeEscalationAllowed {
    fn meta(&self) -> &'static RuleMeta {
        &META
    }

    fn check_object(&self, cx: &mut ObjectContext<'_>) {
        let Some(pod) = cx.object().pod_spec().filter(|pod| !pod.is_windows()) else {
            return;
        };
        for container in pod.containers() {
            let context = container.security_context();
            let setting = |path: &[&str]| context.and_then(|context| context.get_path(path));
            if setting(&["privileged"]).and_then(Node::as_bool) == Some(true)
                || setting(&["capabilities", "add"])
                    .into_iter()
                    .flat_map(Node::items)
                    .filter_map(Node::as_str)
                    .any(|capability| matches!(capability, "SYS_ADMIN" | "CAP_SYS_ADMIN"))
            {
                continue;
            }
            let entry = context.and_then(|context| context.get_entry("allowPrivilegeEscalation"));
            let (range, problem) = match entry {
                Some(entry) if entry.value.as_bool() == Some(true) => (
                    entry.key.range.cover(entry.value.range),
                    "allows privilege escalation",
                ),
                Some(entry) if !entry.value.is_null() => continue,
                _ => (
                    container_range(container),
                    "does not set `allowPrivilegeEscalation: false`",
                ),
            };
            cx.report(
                cx.diagnostic(range, format!("{} {problem}", container_subject(container)))
                    .with_help("set `securityContext.allowPrivilegeEscalation: false`"),
            );
        }
    }
}
