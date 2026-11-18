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

/// The default for `--kubernetes-version`.
pub const LATEST_VERSION: &str = VERSIONS[VERSIONS.len() - 1];

static PARSED: [OnceLock<Schema>; VERSIONS.len()] = [const { OnceLock::new() }; VERSIONS.len()];

impl Schema {
    /// The embedded schema of a minor version such as `1.37`, parsed on first use.
    /// `None` for a version outside [`VERSIONS`].
    ///
    /// ```
    /// use kubelint_schema::{LATEST_VERSION, Schema};
    ///
    /// assert!(Schema::embedded(LATEST_VERSION).is_some());
    /// assert!(Schema::embedded("1.20").is_none());
    /// ```
    pub fn embedded(version: &str) -> Option<&'static Self> {
        let index = VERSIONS.iter().position(|known| *known == version)?;
        Some(PARSED[index].get_or_init(|| {
            Self::from_json(FILES[index])
                .expect("every embedded schema loads, as tests/embedded.rs checks")
        }))
    }
}
