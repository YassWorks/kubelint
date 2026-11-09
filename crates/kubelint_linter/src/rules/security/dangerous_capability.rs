//! KL2006 `dangerous-capability`.

use std::cell::LazyCell;

use kubelint_diagnostics::Severity;
use kubelint_yaml::Node;

use super::container_subject;
use crate::{Category, ObjectContext, Rule, RuleMeta};

/// Reports each capability in a container's `securityContext.capabilities.add` that the
/// `baseline` Pod Security Standard does not allow.
pub struct DangerousCapability;

static META: RuleMeta = RuleMeta {
    code: "KL2006",
    name: "dangerous-capability",
    category: Category::Security,
    default_severity: Severity::Warning,
    default_enabled: true,
    summary: "A container adds a Linux capability that weakens its isolation.",
    explanation: include_str!("dangerous_capability.md"),
};

/// The capabilities the `baseline` profile allows adding (`check_capabilities_baseline.go` in
/// `k8s.io/pod-security-admission/policy`).
const BASELINE: [&str; 13] = [
    "AUDIT_WRITE",
    "CHOWN",
    "DAC_OVERRIDE",
    "FOWNER",
    "FSETID",
    "KILL",
    "MKNOD",
    "NET_BIND_SERVICE",
    "SETFCAP",
    "SETGID",
    "SETPCAP",
    "SETUID",
    "SYS_CHROOT",
];

impl Rule for DangerousCapability {
    fn meta(&self) -> &'static RuleMeta {
        &META
    }

    fn check_object(&self, cx: &mut ObjectContext<'_>) {
        let Some(pod) = cx.object().pod_spec() else {
            return;
        };
        for container in pod.containers() {
            let subject = LazyCell::new(|| container_subject(container));
            let added = container
                .security_context()
                .and_then(|context| context.get_path(&["capabilities", "add"]))
                .into_iter()
                .flat_map(Node::items)
                .filter_map(|item| Some((item, item.as_str()?)))
                .filter(|(_, text)| !BASELINE.contains(text));
            for (item, text) in added {
                // Pod Security Admission compares exact names; runtimes upper-case them
                // and add the `CAP_` prefix.
                let upper = text.to_ascii_uppercase();
                let name = upper.strip_prefix("CAP_").unwrap_or(&upper);
                let message = if name == "ALL" {
                    format!("{} adds every Linux capability (`{text}`)", *subject)
                } else {
                    format!("{} adds the `{text}` capability", *subject)
                };
                let help = if BASELINE.contains(&name) {
                    format!(
                        "write it as `{name}`: the `baseline` Pod Security Standard allows \
                         only that spelling"
                    )
                } else {
                    "remove it from `capabilities.add`, or silence this finding with the \
                     reason the container needs it"
                        .to_owned()
                };
                cx.report(cx.diagnostic(item.range, message).with_help(help));
            }
        }
    }
}
