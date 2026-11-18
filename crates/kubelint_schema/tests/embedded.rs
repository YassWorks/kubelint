use std::path::Path;

use kubelint_k8s::{GroupVersion, GroupVersionKind};
use kubelint_schema::{Schema, VERSIONS};

fn has_kind(version: &str, api_version: &str, kind: &str) -> bool {
    let schema = Schema::embedded(version).expect("tests use supported versions");
    let group_version = GroupVersion::parse(api_version).expect("tests use valid API versions");
    schema
        .lookup(GroupVersionKind::new(group_version, kind))
        .is_some()
}

#[test]
fn every_schema_file_is_embedded() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("schemas");
    let mut files: Vec<String> = std::fs::read_dir(dir)
        .expect("the schemas directory can be read")
        .filter_map(|entry| {
            let path = entry.expect("a directory entry can be read").path();
            let stem = path.file_stem()?.to_str()?.to_owned();
            path.extension()
                .is_some_and(|extension| extension == "json")
                .then_some(stem)
        })
        .collect();
    files.sort_by_key(|version| {
        version
            .split('.')
            .map(|part| {
                part.parse::<u32>()
                    .expect("schema files are named by version")
            })
            .collect::<Vec<_>>()
    });
    assert_eq!(files, VERSIONS);
}

#[test]
fn every_embedded_schema_loads_once() {
    for version in VERSIONS {
        let first = Schema::embedded(version).expect("a supported version is embedded");
        let second = Schema::embedded(version).expect("a supported version is embedded");
        assert!(std::ptr::eq(first, second), "{version}");
        assert!(has_kind(version, "apps/v1", "Deployment"), "{version}");
        assert!(has_kind(version, "v1", "Pod"), "{version}");
    }
}

#[test]
fn each_version_has_its_own_kinds() {
    assert!(!has_kind(
        "1.34",
        "admissionregistration.k8s.io/v1",
        "MutatingAdmissionPolicy"
    ));
    assert!(has_kind(
        "1.37",
        "admissionregistration.k8s.io/v1",
        "MutatingAdmissionPolicy"
    ));
    assert!(has_kind("1.34", "networking.k8s.io/v1beta1", "ServiceCIDR"));
    assert!(!has_kind(
        "1.37",
        "networking.k8s.io/v1beta1",
        "ServiceCIDR"
    ));
}

#[test]
fn versions_outside_the_window_have_no_schema() {
    for version in ["1.33", "1.38", "v1.37", "1.37.2", ""] {
        assert!(Schema::embedded(version).is_none(), "{version}");
    }
}
