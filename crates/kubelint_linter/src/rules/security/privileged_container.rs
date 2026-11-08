//! KL2001 `privileged-container`.

use kubelint_diagnostics::Severity;

use super::container_subject;
use crate::{Category, ObjectContext, Rule, RuleMeta};

/// Reports a container whose `securityContext.privileged` is true. Kubernetes has no
/// pod-level `privileged`, so only container settings count.
pub struct PrivilegedContainer;

static META: RuleMeta = RuleMeta {
    code: "KL2001",
    name: "privileged-container",
    category: Category::Security,
    default_severity: Severity::Error,
    default_enabled: true,
    summary: "A container runs in privileged mode.",
    explanation: include_str!("privileged_container.md"),
};

impl Rule for PrivilegedContainer {
    fn meta(&self) -> &'static RuleMeta {
        &META
    }

    fn check_object(&self, cx: &mut ObjectContext<'_>) {
        let Some(pod) = cx.object().pod_spec() else {
            return;
        };
        for container in pod.containers() {
            let Some(entry) = container
                .security_context()
                .and_then(|context| context.get_entry("privileged"))
                .filter(|entry| entry.value.as_bool() == Some(true))
            else {
                continue;
            };
            let message = format!("{} runs in privileged mode", container_subject(container));
            cx.report(
                cx.diagnostic(entry.key.range.cover(entry.value.range), message)
                    .with_help(
                        "remove `privileged` and add only the capabilities the container needs \
                         to `securityContext.capabilities.add`",
                    ),
            );
        }
    }
}
