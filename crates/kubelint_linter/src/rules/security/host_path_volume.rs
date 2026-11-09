//! KL2005 `host-path-volume`.

use kubelint_diagnostics::Severity;
use kubelint_yaml::Node;

use crate::{Category, ObjectContext, Rule, RuleMeta};

/// Reports every pod volume of type `hostPath`. A `hostPath` that is not a mapping is a
/// schema problem.
pub struct HostPathVolume;

static META: RuleMeta = RuleMeta {
    code: "KL2005",
    name: "host-path-volume",
    category: Category::Security,
    default_severity: Severity::Warning,
    default_enabled: true,
    summary: "A volume mounts a directory or file of the node.",
    explanation: include_str!("host_path_volume.md"),
};

impl Rule for HostPathVolume {
    fn meta(&self) -> &'static RuleMeta {
        &META
    }

    fn check_object(&self, cx: &mut ObjectContext<'_>) {
        let Some(pod) = cx.object().pod_spec() else {
            return;
        };
        let volumes = pod.node().get("volumes").into_iter().flat_map(Node::items);
        for volume in volumes {
            let Some(host_path) = volume
                .get_entry("hostPath")
                .filter(|entry| entry.value.as_mapping().is_some())
            else {
                continue;
            };
            let subject = match volume.get("name").and_then(Node::as_str) {
                Some(name) => format!("volume `{name}`"),
                None => "volume".to_owned(),
            };
            let path = host_path.value.get_entry("path");
            let message = match path.and_then(|entry| entry.value.as_str()) {
                Some(path) => format!("{subject} mounts `{path}` from the node"),
                None => format!("{subject} mounts a path from the node"),
            };
            let range = path.map_or(host_path.key.range, |entry| {
                entry.key.range.cover(entry.value.range)
            });
            cx.report(cx.diagnostic(range, message).with_help(
                "use a `configMap`, `secret`, `emptyDir` or `persistentVolumeClaim` volume \
                 instead",
            ));
        }
    }
}
