//! KL3002 `missing-resource-requests`.

use kubelint_diagnostics::Severity;
use kubelint_k8s::ContainerKind;
use kubelint_yaml::Entry;

use super::ResourceField;
use crate::rules::{container_range, container_subject};
use crate::{Category, ObjectContext, Rule, RuleMeta};

/// Reports a regular or init container without a CPU or a memory request.
///
/// A limit counts as a request: `SetDefaults_Pod` in `pkg/apis/core/v1/defaults.go`
/// copies it into a missing request. Ephemeral containers cannot set `resources`.
pub struct MissingResourceRequests;

static META: RuleMeta = RuleMeta {
    code: "KL3002",
    name: "missing-resource-requests",
    category: Category::Reliability,
    default_severity: Severity::Warning,
    default_enabled: true,
    summary: "A container has no CPU or memory `requests`.",
    explanation: include_str!("missing_resource_requests.md"),
};

/// A resource that every container should request.
struct Resource {
    key: &'static str,
    label: &'static str,
    example: &'static str,
}

const RESOURCES: [Resource; 2] = [
    Resource {
        key: "cpu",
        label: "CPU",
        example: "cpu: 100m",
    },
    Resource {
        key: "memory",
        label: "memory",
        example: "memory: 256Mi",
    },
];

impl Rule for MissingResourceRequests {
    fn meta(&self) -> &'static RuleMeta {
        &META
    }

    fn check_object(&self, cx: &mut ObjectContext<'_>) {
        let Some(pod) = cx.object().pod_spec() else {
            return;
        };
        let pod_resources = pod.node().get_entry("resources");
        let uncovered: Vec<&Resource> = RESOURCES
            .iter()
            .filter(|resource| !requested(pod_resources, resource.key))
            .collect();
        if uncovered.is_empty() {
            return;
        }
        let containers = pod
            .containers()
            .filter(|container| container.kind() != ContainerKind::Ephemeral);
        for container in containers {
            let resources = container.node().get_entry("resources");
            let missing: Vec<&Resource> = uncovered
                .iter()
                .copied()
                .filter(|resource| !requested(resources, resource.key))
                .collect();
            let [first, ..] = missing.as_slice() else {
                continue;
            };
            let labels: Vec<_> = missing.iter().map(|resource| resource.label).collect();
            let fields: Vec<_> = missing
                .iter()
                .map(|resource| format!("`resources.requests.{}`", resource.key))
                .collect();
            let examples: Vec<_> = missing
                .iter()
                .map(|resource| format!("`{}`", resource.example))
                .collect();
            let plural = if missing.len() > 1 { "s" } else { "" };
            let range = match ResourceField::lookup(resources, "requests", first.key) {
                ResourceField::Missing(Some(range)) => range,
                _ => container_range(container),
            };
            cx.report(
                cx.diagnostic(
                    range,
                    format!(
                        "{} has no {} request{plural}",
                        container_subject(container),
                        labels.join(" or "),
                    ),
                )
                .with_help(format!(
                    "set {}, for example {}",
                    fields.join(" and "),
                    examples.join(" and "),
                )),
            );
        }
    }
}

/// Whether a request or a limit of `resource` is set under `resources`.
fn requested(resources: Option<&Entry>, resource: &str) -> bool {
    ["requests", "limits"]
        .into_iter()
        .any(|section| ResourceField::lookup(resources, section, resource).is_set())
}
