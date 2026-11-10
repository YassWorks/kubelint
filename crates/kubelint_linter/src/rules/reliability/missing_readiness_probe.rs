//! KL3004 `missing-readiness-probe`.

use kubelint_diagnostics::Severity;
use kubelint_k8s::{BuiltinKind, ContainerKind};
use kubelint_yaml::Node;

use crate::rules::{container_range, container_subject};
use crate::{Category, ObjectContext, Rule, RuleMeta};

/// Reports a regular container of a long-running workload without a `readinessProbe`.
///
/// The API server rejects `readinessProbe` on init containers that are not sidecars and
/// on ephemeral containers (`pkg/apis/core/validation/validation.go`). Sidecars are
/// skipped by choice, as recorded in `docs/rules/README.md`.
pub struct MissingReadinessProbe;

static META: RuleMeta = RuleMeta {
    code: "KL3004",
    name: "missing-readiness-probe",
    category: Category::Reliability,
    default_severity: Severity::Info,
    default_enabled: false,
    summary: "A long-running workload container has no `readinessProbe`.",
    explanation: include_str!("missing_readiness_probe.md"),
};

impl Rule for MissingReadinessProbe {
    fn meta(&self) -> &'static RuleMeta {
        &META
    }

    fn check_object(&self, cx: &mut ObjectContext<'_>) {
        if !matches!(
            cx.object().builtin_kind(),
            Some(
                BuiltinKind::Deployment
                    | BuiltinKind::StatefulSet
                    | BuiltinKind::DaemonSet
                    | BuiltinKind::ReplicaSet
                    | BuiltinKind::ReplicationController
            )
        ) {
            return;
        }
        let Some(pod) = cx.object().pod_spec() else {
            return;
        };
        let containers = pod.containers().filter(|container| {
            container.kind() == ContainerKind::Regular
                && container
                    .readiness_probe()
                    .is_none_or(Node::is_kubernetes_null)
        });
        for container in containers {
            cx.report(
                cx.diagnostic(
                    container_range(container),
                    format!("{} has no readiness probe", container_subject(container)),
                )
                .with_help(
                    "add a `readinessProbe`, such as an `httpGet` on a health endpoint, so \
                     the container gets traffic only once it can serve it",
                ),
            );
        }
    }
}
