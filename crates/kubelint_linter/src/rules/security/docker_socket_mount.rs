//! KL2007 `docker-socket-mount`.

use kubelint_diagnostics::Severity;

use super::HostPathMount;
use crate::{Category, ObjectContext, Rule, RuleMeta};

/// Reports a `hostPath` volume whose path ends in a container runtime socket file name.
/// Directories that hold a socket (`/var/run`) are not reported.
pub struct DockerSocketMount;

static META: RuleMeta = RuleMeta {
    code: "KL2007",
    name: "docker-socket-mount",
    category: Category::Security,
    default_severity: Severity::Error,
    default_enabled: true,
    summary: "A volume mounts the container runtime socket of the node.",
    explanation: include_str!("docker_socket_mount.md"),
};

impl Rule for DockerSocketMount {
    fn meta(&self) -> &'static RuleMeta {
        &META
    }

    fn check_object(&self, cx: &mut ObjectContext<'_>) {
        let Some(pod) = cx.object().pod_spec() else {
            return;
        };
        let sockets =
            HostPathMount::all(pod).filter_map(|mount| Some((mount, mount.runtime_socket()?)));
        for (mount, path) in sockets {
            let message = format!(
                "{} mounts the container runtime socket `{path}`",
                mount.subject()
            );
            cx.report(cx.diagnostic(mount.range(), message).with_help(
                "remove the volume; to build images in the cluster, use a builder that needs \
                 no daemon, such as BuildKit in rootless mode or Kaniko",
            ));
        }
    }
}
