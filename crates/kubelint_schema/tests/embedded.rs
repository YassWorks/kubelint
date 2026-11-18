use std::path::Path;

use kubelint_k8s::{GroupVersion, GroupVersionKind};
use kubelint_schema::{KubernetesVersion, Schema, VERSIONS};

fn has_kind(version: &str, api_version: &str, kind: &str) -> bool {
    let version = KubernetesVersion::parse(version).expect("tests use supported versions");
    let schema = Schema::embedded(version);
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
    assert_eq!(KubernetesVersion::LATEST.as_str(), files[files.len() - 1]);
}

#[test]
fn every_embedded_schema_loads_once() {
    for version in VERSIONS {
        let parsed = KubernetesVersion::parse(version).expect("every listed version parses");
        assert_eq!(parsed.as_str(), *version);
        let first = Schema::embedded(parsed);
        let second = Schema::embedded(parsed);
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
    for version in ["1.33", "1.38", "1.20", ""] {
        assert_eq!(KubernetesVersion::parse(version), None, "{version}");
    }
}

#[test]
fn supported_versions_are_named_with_or_without_v_and_patch() {
    for (text, expected) in [
        ("1.37", Some("1.37")),
        ("v1.37", Some("1.37")),
        ("1.37.2", Some("1.37")),
        ("v1.34.0", Some("1.34")),
        ("1.33", None),
        ("1.38", None),
        ("1.3", None),
        ("1.370", None),
        ("1.37.", None),
        ("1.37.x", None),
        ("1.37.2.1", None),
        ("vv1.37", None),
        (" 1.37", None),
        ("V1.37", None),
        ("", None),
    ] {
        let parsed = KubernetesVersion::parse(text).map(KubernetesVersion::as_str);
        assert_eq!(parsed, expected, "{text:?}");
    }
}
