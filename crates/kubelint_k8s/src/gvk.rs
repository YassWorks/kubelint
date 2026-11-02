//! `apiVersion` and `kind` as Kubernetes understands them.

use std::fmt;

/// An API group and version, parsed from an `apiVersion` value such as `apps/v1`.
///
/// The core group (Pod, Service, `ConfigMap`...) has an empty name and is written as
/// just the version: `apiVersion: v1`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct GroupVersion<'a> {
    /// The API group, empty for the core group.
    pub group: &'a str,
    /// The version, such as `v1` or `v1beta1`.
    pub version: &'a str,
}

impl<'a> GroupVersion<'a> {
    /// Parses an `apiVersion` value the way Kubernetes' `schema.ParseGroupVersion` does:
    /// no `/` means the core group, one `/` separates group and version, more than one
    /// is invalid. Unlike Kubernetes, an empty string or an empty part is also rejected,
    /// because a manifest with `apiVersion: ""` or `apiVersion: apps/` cannot be applied.
    ///
    /// Source: `staging/src/k8s.io/apimachinery/pkg/runtime/schema/group_version.go`.
    ///
    /// ```
    /// use kubelint_k8s::GroupVersion;
    ///
    /// let core = GroupVersion::parse("v1").unwrap();
    /// assert_eq!((core.group, core.version), ("", "v1"));
    /// let apps = GroupVersion::parse("apps/v1").unwrap();
    /// assert_eq!((apps.group, apps.version), ("apps", "v1"));
    /// assert!(GroupVersion::parse("a/b/c").is_none());
    /// ```
    pub fn parse(api_version: &'a str) -> Option<Self> {
        let (group, version) = match api_version.split_once('/') {
            None => ("", api_version),
            Some((group, version)) if !group.is_empty() && !version.contains('/') => {
                (group, version)
            }
            Some(_) => return None,
        };
        (!version.is_empty()).then_some(Self { group, version })
    }

    /// Whether this is the core (legacy) group, written without a group name.
    pub fn is_core(&self) -> bool {
        self.group.is_empty()
    }
}

/// The stable `apiVersion` of a built-in kind, for help texts such as "add
/// `apiVersion: apps/v1`". `None` for custom resources and kinds not in the table.
///
/// The table lists the GA (non-beta) versions served by every Kubernetes release since
/// 1.25. Kind names are case-sensitive, as in Kubernetes.
///
/// ```
/// use kubelint_k8s::stable_api_version;
///
/// assert_eq!(stable_api_version("Deployment"), Some("apps/v1"));
/// assert_eq!(stable_api_version("Pod"), Some("v1"));
/// assert_eq!(stable_api_version("Certificate"), None);
/// ```
pub fn stable_api_version(kind: &str) -> Option<&'static str> {
    let api_version = match kind {
        "ConfigMap"
        | "Endpoints"
        | "LimitRange"
        | "Namespace"
        | "PersistentVolume"
        | "PersistentVolumeClaim"
        | "Pod"
        | "PodTemplate"
        | "ReplicationController"
        | "ResourceQuota"
        | "Secret"
        | "Service"
        | "ServiceAccount" => "v1",
        "ControllerRevision" | "DaemonSet" | "Deployment" | "ReplicaSet" | "StatefulSet" => {
            "apps/v1"
        }
        "CronJob" | "Job" => "batch/v1",
        "HorizontalPodAutoscaler" => "autoscaling/v2",
        "Ingress" | "IngressClass" | "NetworkPolicy" => "networking.k8s.io/v1",
        "ClusterRole" | "ClusterRoleBinding" | "Role" | "RoleBinding" => {
            "rbac.authorization.k8s.io/v1"
        }
        "PodDisruptionBudget" => "policy/v1",
        "CSIDriver" | "StorageClass" | "VolumeAttachment" => "storage.k8s.io/v1",
        "PriorityClass" => "scheduling.k8s.io/v1",
        "CustomResourceDefinition" => "apiextensions.k8s.io/v1",
        "MutatingWebhookConfiguration" | "ValidatingWebhookConfiguration" => {
            "admissionregistration.k8s.io/v1"
        }
        "Lease" => "coordination.k8s.io/v1",
        _ => return None,
    };
    Some(api_version)
}

impl fmt::Display for GroupVersion<'_> {
    /// Formats back to the `apiVersion` spelling.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.is_core() {
            f.write_str(self.version)
        } else {
            write!(f, "{}/{}", self.group, self.version)
        }
    }
}

/// A group, version and kind: what identifies the type of a Kubernetes object.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct GroupVersionKind<'a> {
    /// The API group, empty for the core group.
    pub group: &'a str,
    /// The version, such as `v1`.
    pub version: &'a str,
    /// The kind, such as `Deployment`. Case-sensitive, as in Kubernetes.
    pub kind: &'a str,
}

impl<'a> GroupVersionKind<'a> {
    /// Combines a parsed `apiVersion` with a `kind`.
    ///
    /// ```
    /// use kubelint_k8s::{GroupVersion, GroupVersionKind};
    ///
    /// let gv = GroupVersion::parse("apps/v1").unwrap();
    /// let gvk = GroupVersionKind::new(gv, "Deployment");
    /// assert_eq!(gvk.to_string(), "apps/v1 Deployment");
    /// ```
    pub fn new(group_version: GroupVersion<'a>, kind: &'a str) -> Self {
        Self {
            group: group_version.group,
            version: group_version.version,
            kind,
        }
    }

    /// The group and version part.
    pub fn group_version(&self) -> GroupVersion<'a> {
        GroupVersion {
            group: self.group,
            version: self.version,
        }
    }
}

impl fmt::Display for GroupVersionKind<'_> {
    /// Formats as `apiVersion kind`, for messages.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} {}", self.group_version(), self.kind)
    }
}

#[cfg(test)]
mod tests {
    use super::GroupVersion;

    #[test]
    fn parses_api_versions() {
        let cases = [
            ("v1", Some(("", "v1"))),
            ("apps/v1", Some(("apps", "v1"))),
            ("networking.k8s.io/v1", Some(("networking.k8s.io", "v1"))),
            ("batch/v1beta1", Some(("batch", "v1beta1"))),
            ("", None),
            ("apps/", None),
            ("/v1", None),
            ("a/b/c", None),
            ("/", None),
        ];
        for (input, expected) in cases {
            let got = GroupVersion::parse(input).map(|gv| (gv.group, gv.version));
            assert_eq!(got, expected, "input {input:?}");
        }
    }

    #[test]
    fn display_round_trips() {
        for input in ["v1", "apps/v1", "rbac.authorization.k8s.io/v1"] {
            let gv = GroupVersion::parse(input).map(|gv| gv.to_string());
            assert_eq!(gv.as_deref(), Some(input));
        }
    }
}
