//! KL2005 `host-path-volume`.

use kubelint_diagnostics::Severity;

use super::HostPathMount;
use crate::{Category, ObjectContext, Rule, RuleMeta};

/// Reports every pod volume of type `hostPath`, except the container runtime sockets
/// KL2007 reports.
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
        for mount in HostPathMount::all(pod).filter(|mount| mount.runtime_socket().is_none()) {
            let message = match mount.path() {
                Some(path) => format!("{} mounts `{path}` from the node", mount.subject()),
                None => format!("{} mounts a path from the node", mount.subject()),
            };
            cx.report(cx.diagnostic(mount.range(), message).with_help(
                "use a `configMap`, `secret`, `emptyDir` or `persistentVolumeClaim` volume \
                 instead",
            ));
        }
    }
}
