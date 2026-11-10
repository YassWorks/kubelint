//! KL2009 `writable-root-filesystem`.

use kubelint_diagnostics::Severity;

use super::{container_range, container_subject};
use crate::{Category, ObjectContext, Rule, RuleMeta};

/// Reports a container that does not set `securityContext.readOnlyRootFilesystem` to
/// true. Unset means writable, and there is no pod-level setting.
///
/// The API server rejects the field on Windows pods (`validateWindows` in
/// `pkg/apis/core/validation/validation.go`), so those are skipped. A value that is
/// neither a boolean nor null is a schema problem.
pub struct WritableRootFilesystem;

static META: RuleMeta = RuleMeta {
    code: "KL2009",
    name: "writable-root-filesystem",
    category: Category::Security,
    default_severity: Severity::Warning,
    default_enabled: false,
    summary: "A container does not set `readOnlyRootFilesystem: true`.",
    explanation: include_str!("writable_root_filesystem.md"),
};

impl Rule for WritableRootFilesystem {
    fn meta(&self) -> &'static RuleMeta {
        &META
    }

    fn check_object(&self, cx: &mut ObjectContext<'_>) {
        let Some(pod) = cx.object().pod_spec().filter(|pod| !pod.is_windows()) else {
            return;
        };
        for container in pod.containers() {
            let entry = container
                .security_context()
                .and_then(|context| context.get_entry("readOnlyRootFilesystem"));
            let (range, problem) = match entry {
                Some(entry) if entry.value.as_bool() == Some(false) => (
                    entry.key.range.cover(entry.value.range),
                    "has a writable root filesystem",
                ),
                Some(entry) if !entry.value.is_kubernetes_null() => continue,
                _ => (
                    container_range(container),
                    "does not set `readOnlyRootFilesystem: true`",
                ),
            };
            cx.report(
                cx.diagnostic(range, format!("{} {problem}", container_subject(container)))
                    .with_help(
                        "set `securityContext.readOnlyRootFilesystem: true`, and mount an \
                         `emptyDir` volume on each path the container writes to",
                    ),
            );
        }
    }
}
