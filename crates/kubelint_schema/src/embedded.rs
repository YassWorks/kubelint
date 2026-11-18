use std::fmt;
use std::sync::OnceLock;

use crate::Schema;

macro_rules! embed {
    ($($version:literal),+) => {
        /// The Kubernetes minor versions kubelint has a schema for, oldest first.
        pub const VERSIONS: &[&str] = &[$($version),+];
        const FILES: &[&str] = &[$(include_str!(concat!("../schemas/", $version, ".json"))),+];
    };
}

embed!("1.34", "1.35", "1.36", "1.37");

static PARSED: [OnceLock<Schema>; VERSIONS.len()] = [const { OnceLock::new() }; VERSIONS.len()];

/// One of [`VERSIONS`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KubernetesVersion(usize);

impl KubernetesVersion {
    /// The default for `--kubernetes-version`.
    pub const LATEST: Self = Self(VERSIONS.len() - 1);

    /// The supported version that `text` names: a minor version with an optional `v` and
    /// an optional patch number, which is ignored (`1.37`, `v1.37`, `1.37.2`).
    ///
    /// ```
    /// use kubelint_schema::KubernetesVersion;
    ///
    /// let version = KubernetesVersion::parse("v1.37.2").unwrap();
    /// assert_eq!(version.as_str(), "1.37");
    /// assert_eq!(KubernetesVersion::parse("1.20"), None);
    /// ```
    pub fn parse(text: &str) -> Option<Self> {
        let text = text.strip_prefix('v').unwrap_or(text);
        let is_patch = |rest: &str| {
            rest.strip_prefix('.').is_some_and(|patch| {
                !patch.is_empty() && patch.bytes().all(|byte| byte.is_ascii_digit())
            })
        };
        VERSIONS
            .iter()
            .position(|version| {
                text.strip_prefix(version)
                    .is_some_and(|rest| rest.is_empty() || is_patch(rest))
            })
            .map(Self)
    }

    pub fn as_str(self) -> &'static str {
        VERSIONS[self.0]
    }
}

impl fmt::Display for KubernetesVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl Schema {
    /// The embedded schema of `version`, parsed on first use.
    pub fn embedded(version: KubernetesVersion) -> &'static Self {
        PARSED[version.0].get_or_init(|| {
            Self::from_json(FILES[version.0])
                .expect("every embedded schema loads, as tests/embedded.rs checks")
        })
    }
}
