//! KL3001 `missing-memory-limit`.

use kubelint_diagnostics::Severity;
use kubelint_k8s::ContainerKind;

use super::ResourceField;
use crate::rules::{container_range, container_subject};
use crate::{Category, ObjectContext, Rule, RuleMeta};

/// Reports a regular or init container without `resources.limits.memory`.
///
/// The API server rejects `resources` on ephemeral containers
/// (`allowedEphemeralContainerFields` in `pkg/apis/core/validation/validation.go`), so
/// they are skipped. A `resources` or `limits` value that is neither a mapping nor null
/// is a schema problem and is not reported.
pub struct MissingMemoryLimit;

static META: RuleMeta = RuleMeta {
    code: "KL3001",
    name: "missing-memory-limit",
    category: Category::Reliability,
    default_severity: Severity::Warning,
    default_enabled: true,
    summary: "A container has no `resources.limits.memory`.",
    explanation: include_str!("missing_memory_limit.md"),
};

impl Rule for MissingMemoryLimit {
    fn meta(&self) -> &'static RuleMeta {
        &META
    }

    fn check_object(&self, cx: &mut ObjectContext<'_>) {
        let Some(pod) = cx.object().pod_spec() else {
            return;
        };
        if ResourceField::lookup(pod.node().get_entry("resources"), "limits", "memory").is_set() {
            return;
        }
        let containers = pod
            .containers()
            .filter(|container| container.kind() != ContainerKind::Ephemeral);
        for container in containers {
            let resources = container.node().get_entry("resources");
            let ResourceField::Missing(range) =
                ResourceField::lookup(resources, "limits", "memory")
            else {
                continue;
            };
            cx.report(
                cx.diagnostic(
                    range.unwrap_or_else(|| container_range(container)),
                    format!("{} has no memory limit", container_subject(container)),
                )
                .with_help("set `resources.limits.memory`, for example `memory: 512Mi`"),
            );
        }
    }
}
