//! KL2004 `host-namespace`.

use kubelint_diagnostics::Severity;

use crate::{Category, ObjectContext, Rule, RuleMeta};

/// Reports a pod spec that sets `hostNetwork`, `hostPID` or `hostIPC` to true. These are
/// pod-level fields only. Values of the wrong type are schema problems.
pub struct HostNamespace;

static META: RuleMeta = RuleMeta {
    code: "KL2004",
    name: "host-namespace",
    category: Category::Security,
    default_severity: Severity::Warning,
    default_enabled: true,
    summary: "A pod shares a namespace with its node.",
    explanation: include_str!("host_namespace.md"),
};

/// Each field, the namespace it shares, and the help.
const FIELDS: [(&str, &str, &str); 3] = [
    (
        "hostNetwork",
        "network",
        "remove `hostNetwork` and reach the pod through a Service",
    ),
    (
        "hostPID",
        "process",
        "remove `hostPID`; to share processes between the containers of one pod, set \
         `shareProcessNamespace: true`",
    ),
    (
        "hostIPC",
        "IPC",
        "remove `hostIPC`; the containers of one pod already share an IPC namespace",
    ),
];

impl Rule for HostNamespace {
    fn meta(&self) -> &'static RuleMeta {
        &META
    }

    fn check_object(&self, cx: &mut ObjectContext<'_>) {
        let Some(pod) = cx.object().pod_spec() else {
            return;
        };
        for (field, namespace, help) in FIELDS {
            let Some(entry) = pod
                .node()
                .get_entry(field)
                .filter(|entry| entry.value.as_bool() == Some(true))
            else {
                continue;
            };
            let message = format!("the pod shares the node's {namespace} namespace (`{field}`)");
            cx.report(
                cx.diagnostic(entry.key.range.cover(entry.value.range), message)
                    .with_help(help),
            );
        }
    }
}
