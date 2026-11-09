//! `KL2xxx`: settings that weaken the isolation between a container and its node.

mod dangerous_capability;
mod docker_socket_mount;
mod host_namespace;
mod host_path_volume;
mod privilege_escalation_allowed;
mod privileged_container;
mod run_as_root;
mod secret_in_env_literal;

pub use dangerous_capability::DangerousCapability;
pub use docker_socket_mount::DockerSocketMount;
pub use host_namespace::HostNamespace;
pub use host_path_volume::HostPathVolume;
pub use privilege_escalation_allowed::PrivilegeEscalationAllowed;
pub use privileged_container::PrivilegedContainer;
pub use run_as_root::RunAsRoot;
pub use secret_in_env_literal::SecretInEnvLiteral;

use kubelint_k8s::{Container, PodSpec};
use kubelint_source::TextRange;
use kubelint_yaml::{Entry, Node};

/// "container `web`", or "container" when it has no usable name.
fn container_subject(container: Container<'_>) -> String {
    match container.name().and_then(Node::as_str) {
        Some(name) => format!("container `{name}`"),
        None => "container".to_owned(),
    }
}

/// Where a finding about a missing setting of `container` points: its `name`, or its
/// first key when it has none.
fn container_range(container: Container<'_>) -> TextRange {
    container
        .name()
        .or_else(|| container.node().entries().next().map(|entry| &entry.key))
        .map_or(container.node().range, |node| node.range)
}

/// File names of the container runtime sockets: Docker, cri-dockerd, containerd, CRI-O
/// and Podman.
const RUNTIME_SOCKETS: [&str; 5] = [
    "docker.sock",
    "cri-dockerd.sock",
    "containerd.sock",
    "crio.sock",
    "podman.sock",
];

/// A `hostPath` volume of a pod spec. A `hostPath` that is not a mapping is a schema
/// problem and is skipped.
#[derive(Clone, Copy)]
struct HostPathMount<'a> {
    volume: &'a Node,
    host_path: &'a Entry,
}

impl<'a> HostPathMount<'a> {
    fn all(pod: PodSpec<'a>) -> impl Iterator<Item = Self> {
        pod.volumes().filter_map(|volume| {
            let host_path = volume
                .get_entry("hostPath")
                .filter(|entry| entry.value.as_mapping().is_some())?;
            Some(Self { volume, host_path })
        })
    }

    /// "volume `data`", or "volume" when it has no usable name.
    fn subject(self) -> String {
        match self.volume.get("name").and_then(Node::as_str) {
            Some(name) => format!("volume `{name}`"),
            None => "volume".to_owned(),
        }
    }

    fn path(self) -> Option<&'a str> {
        self.host_path.value.get("path").and_then(Node::as_str)
    }

    /// The `path` entry, or the `hostPath` key when there is no `path`.
    fn range(self) -> TextRange {
        self.host_path
            .value
            .get_entry("path")
            .map_or(self.host_path.key.range, |entry| {
                entry.key.range.cover(entry.value.range)
            })
    }

    /// The `path`, when it names a container runtime socket.
    fn runtime_socket(self) -> Option<&'a str> {
        self.path().filter(|path| {
            path.trim_end_matches('/')
                .rsplit('/')
                .next()
                .is_some_and(|file| RUNTIME_SOCKETS.contains(&file))
        })
    }
}
