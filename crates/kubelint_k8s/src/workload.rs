//! Pod specs and containers inside every workload kind.
//!
//! Most security and reliability rules are "for every container, check X". These
//! helpers find the pod template wherever a workload kind keeps it, so rules never walk
//! `spec.template.spec` by hand and every kind is covered the same way.

use kubelint_yaml::{Entry, Node};

use crate::{BuiltinKind, Object};

/// Where each workload kind keeps its pod spec, and the API groups it is served from.
///
/// Old groups (`extensions`, `apps/v1beta*`, `batch/v1beta1`) are included because
/// manifests written for them still circulate; their pod specs have the same shape.
/// Source: the Kubernetes API reference, "Workload Resources".
const WORKLOADS: &[(BuiltinKind, &[&str], &[&str])] = &[
    (BuiltinKind::Pod, &[""], &["spec"]),
    (
        BuiltinKind::ReplicationController,
        &[""],
        &["spec", "template", "spec"],
    ),
    (
        BuiltinKind::Deployment,
        &["apps", "extensions"],
        &["spec", "template", "spec"],
    ),
    (
        BuiltinKind::ReplicaSet,
        &["apps", "extensions"],
        &["spec", "template", "spec"],
    ),
    (
        BuiltinKind::DaemonSet,
        &["apps", "extensions"],
        &["spec", "template", "spec"],
    ),
    (
        BuiltinKind::StatefulSet,
        &["apps"],
        &["spec", "template", "spec"],
    ),
    (BuiltinKind::Job, &["batch"], &["spec", "template", "spec"]),
    (
        BuiltinKind::CronJob,
        &["batch"],
        &["spec", "jobTemplate", "spec", "template", "spec"],
    ),
];

impl<'a> Object<'a> {
    /// The pod spec of a workload: Pod, Deployment, StatefulSet, DaemonSet, ReplicaSet,
    /// ReplicationController, Job or CronJob.
    ///
    /// Returns `None` for other kinds, for a kind with the same name in another API
    /// group (a custom resource called `Deployment` is not a Deployment), and when the
    /// pod spec is missing or is not a mapping. When `apiVersion` is missing or invalid
    /// the kind alone decides, so container rules still run while KL1001 reports the
    /// `apiVersion` problem.
    ///
    /// ```
    /// let parse = kubelint_yaml::parse(
    ///     "apiVersion: batch/v1\nkind: CronJob\nspec:\n  jobTemplate:\n    spec:\n      template:\n        spec:\n          containers: []\n",
    /// );
    /// let object = kubelint_k8s::objects(&parse.documents[0])[0];
    /// assert!(object.pod_spec().is_some());
    /// ```
    pub fn pod_spec(self) -> Option<PodSpec<'a>> {
        let kind = BuiltinKind::parse(self.kind()?.as_str()?)?;
        let (_, groups, path) = WORKLOADS.iter().find(|(k, _, _)| *k == kind)?;
        if let Some(gv) = self.group_version()
            && !groups.contains(&gv.group)
        {
            return None;
        }
        let node = self.node().get_path(path)?;
        node.as_mapping().is_some().then_some(PodSpec { node })
    }
}

/// A view of a pod spec (`PodSpec` in the Kubernetes API), wherever it was found.
#[derive(Debug, Clone, Copy)]
pub struct PodSpec<'a> {
    node: &'a Node,
}

impl<'a> PodSpec<'a> {
    pub fn node(self) -> &'a Node {
        self.node
    }

    /// The pod-level `securityContext`. Container settings override it field by field.
    pub fn security_context(self) -> Option<&'a Node> {
        self.node.get("securityContext")
    }

    /// Whether the pod declares `spec.os.name: windows`. Some Linux security fields are
    /// rejected for such pods.
    ///
    /// ```
    /// let parse = kubelint_yaml::parse("kind: Pod\nspec:\n  os: {name: windows}\n");
    /// let pod = kubelint_k8s::objects(&parse.documents[0])[0].pod_spec().unwrap();
    /// assert!(pod.is_windows());
    /// ```
    pub fn is_windows(self) -> bool {
        self.node.get_path(&["os", "name"]).and_then(Node::as_str) == Some("windows")
    }

    /// Every container of the pod: `containers`, then `initContainers`, then
    /// `ephemeralContainers`, each in source order. Items that are not mappings are
    /// skipped.
    ///
    /// ```
    /// use kubelint_k8s::ContainerKind;
    ///
    /// let parse = kubelint_yaml::parse(
    ///     "kind: Pod\nspec:\n  initContainers: [{name: init}]\n  containers: [{name: app}]\n",
    /// );
    /// let pod = kubelint_k8s::objects(&parse.documents[0])[0].pod_spec().unwrap();
    /// let found: Vec<_> = pod
    ///     .containers()
    ///     .map(|c| (c.kind(), c.name().and_then(|n| n.as_str())))
    ///     .collect();
    /// assert_eq!(
    ///     found,
    ///     [(ContainerKind::Regular, Some("app")), (ContainerKind::Init, Some("init"))],
    /// );
    /// ```
    pub fn containers(self) -> impl Iterator<Item = Container<'a>> {
        ContainerKind::ALL.into_iter().flat_map(move |kind| {
            self.node
                .get(kind.field())
                .into_iter()
                .flat_map(Node::items)
                .filter(|item| item.as_mapping().is_some())
                .map(move |node| Container {
                    node,
                    kind,
                    pod: self,
                })
        })
    }
}

/// Which list of the pod spec a container comes from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ContainerKind {
    /// `containers`: the application containers.
    Regular,
    /// `initContainers`: run to completion, one after another, before the others start.
    Init,
    /// `ephemeralContainers`: debugging containers added to a running pod.
    Ephemeral,
}

impl ContainerKind {
    const ALL: [Self; 3] = [Self::Regular, Self::Init, Self::Ephemeral];

    /// The pod spec field holding this kind of container, for messages.
    pub fn field(self) -> &'static str {
        match self {
            Self::Regular => "containers",
            Self::Init => "initContainers",
            Self::Ephemeral => "ephemeralContainers",
        }
    }
}

/// A view of one container in a pod spec.
///
/// Accessors return the field's YAML node, like [`Object`]'s.
#[derive(Debug, Clone, Copy)]
pub struct Container<'a> {
    node: &'a Node,
    kind: ContainerKind,
    pod: PodSpec<'a>,
}

impl<'a> Container<'a> {
    pub fn node(self) -> &'a Node {
        self.node
    }

    pub fn kind(self) -> ContainerKind {
        self.kind
    }

    pub fn name(self) -> Option<&'a Node> {
        self.node.get("name")
    }

    pub fn image(self) -> Option<&'a Node> {
        self.node.get("image")
    }

    pub fn security_context(self) -> Option<&'a Node> {
        self.node.get("securityContext")
    }

    /// The `securityContext` field that applies to this container: its own when set and
    /// not null, otherwise the pod's (`DetermineEffectiveSecurityContext` in
    /// `pkg/securitycontext/util.go`).
    ///
    /// ```
    /// use kubelint_k8s::SecurityField;
    ///
    /// let parse = kubelint_yaml::parse(
    ///     "kind: Pod\nspec: {securityContext: {runAsUser: 1, runAsNonRoot: true}, containers: [{securityContext: {runAsUser: 0, runAsNonRoot: null}}]}",
    /// );
    /// let pod = kubelint_k8s::objects(&parse.documents[0])[0].pod_spec().unwrap();
    /// let container = pod.containers().next().unwrap();
    /// let value = |field| container.security_setting(field).and_then(|e| e.value.as_str());
    /// assert_eq!(value(SecurityField::RunAsUser), Some("0"));
    /// assert_eq!(value(SecurityField::RunAsNonRoot), Some("true"));
    /// ```
    pub fn security_setting(self, field: SecurityField) -> Option<&'a Entry> {
        let set = |context: Option<&'a Node>| {
            context
                .and_then(|context| context.get_entry(field.key()))
                .filter(|entry| !entry.value.is_kubernetes_null())
        };
        set(self.security_context()).or_else(|| set(self.pod.security_context()))
    }

    /// The `resources` value (requests and limits).
    pub fn resources(self) -> Option<&'a Node> {
        self.node.get("resources")
    }

    /// The `env` value (normally a sequence of `name`/`value` mappings).
    pub fn env(self) -> Option<&'a Node> {
        self.node.get("env")
    }

    pub fn liveness_probe(self) -> Option<&'a Node> {
        self.node.get("livenessProbe")
    }

    pub fn readiness_probe(self) -> Option<&'a Node> {
        self.node.get("readinessProbe")
    }

    pub fn startup_probe(self) -> Option<&'a Node> {
        self.node.get("startupProbe")
    }
}

/// The `securityContext` fields a pod sets for all its containers and a container can
/// override.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SecurityField {
    RunAsUser,
    RunAsGroup,
    RunAsNonRoot,
    SeLinuxOptions,
    SeccompProfile,
    AppArmorProfile,
    WindowsOptions,
}

impl SecurityField {
    pub fn key(self) -> &'static str {
        match self {
            Self::RunAsUser => "runAsUser",
            Self::RunAsGroup => "runAsGroup",
            Self::RunAsNonRoot => "runAsNonRoot",
            Self::SeLinuxOptions => "seLinuxOptions",
            Self::SeccompProfile => "seccompProfile",
            Self::AppArmorProfile => "appArmorProfile",
            Self::WindowsOptions => "windowsOptions",
        }
    }
}
