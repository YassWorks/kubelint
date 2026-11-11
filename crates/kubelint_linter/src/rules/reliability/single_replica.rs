//! KL3005 `single-replica`.

use kubelint_diagnostics::Severity;
use kubelint_k8s::BuiltinKind;
use kubelint_yaml::ResolvedScalar;

use crate::{Category, ObjectContext, Rule, RuleMeta};

/// Reports a Deployment or StatefulSet whose `spec.replicas` is the integer 1. An unset
/// `replicas` is skipped, as recorded in `docs/rules/README.md`.
pub struct SingleReplica;

static META: RuleMeta = RuleMeta {
    code: "KL3005",
    name: "single-replica",
    category: Category::Reliability,
    default_severity: Severity::Info,
    default_enabled: false,
    summary: "A Deployment or StatefulSet sets `replicas: 1`.",
    explanation: include_str!("single_replica.md"),
};

impl Rule for SingleReplica {
    fn meta(&self) -> &'static RuleMeta {
        &META
    }

    fn check_object(&self, cx: &mut ObjectContext<'_>) {
        let Some(kind @ (BuiltinKind::Deployment | BuiltinKind::StatefulSet)) =
            cx.object().builtin_kind()
        else {
            return;
        };
        let Some(entry) = cx
            .object()
            .node()
            .get("spec")
            .and_then(|spec| spec.get_entry("replicas"))
            .filter(|entry| {
                entry
                    .value
                    .as_scalar()
                    .is_some_and(|scalar| scalar.resolve_kubernetes() == ResolvedScalar::Int(1))
            })
        else {
            return;
        };
        cx.report(
            cx.diagnostic(
                entry.key.range.cover(entry.value.range),
                format!("the {} runs a single replica", kind.as_str()),
            )
            .with_help(
                "set `replicas` to 2 or more so that one pod failure, node drain or \
                 rollout does not take the workload down",
            ),
        );
    }
}
