//! `Object` views over parsed manifests, most of them fixtures in
//! `resources/test/fixtures/objects/`. Inputs contain non-ASCII text so the ranges in
//! snapshots are byte offsets.

use std::fmt::Write as _;

use kubelint_k8s::{BuiltinKind, objects};
use kubelint_yaml::parse;

/// One block per object: its range and each accessor's value with its range.
fn describe(text: &str) -> String {
    let parse = parse(text);
    assert!(parse.errors.is_empty(), "{:?}", parse.errors);
    let mut out = String::new();
    for (i, doc) in parse.documents.iter().enumerate() {
        for object in objects(doc) {
            let _ = writeln!(out, "doc {i} object {:?}", object.range());
            let fields = [
                ("apiVersion", object.api_version()),
                ("kind", object.kind()),
                ("name", object.name()),
                ("namespace", object.namespace()),
                ("labels", object.labels()),
                ("annotations", object.annotations()),
            ];
            for (field, node) in fields {
                if let Some(node) = node {
                    let shown = node
                        .as_str()
                        .map_or_else(|| "<not a string>".into(), |s| format!("{s:?}"));
                    let _ = writeln!(out, "  {field} {shown} {:?}", node.range);
                }
            }
            match object.gvk() {
                Some(gvk) => {
                    let _ = writeln!(out, "  gvk {gvk:?}");
                }
                None => out.push_str("  gvk none\n"),
            }
        }
    }
    out
}

macro_rules! fixture_tests {
    ($($name:ident),* $(,)?) => {$(
        #[test]
        fn $name() {
            let text = include_str!(concat!(
                "../resources/test/fixtures/objects/",
                stringify!($name),
                ".yaml"
            ));
            insta::assert_snapshot!(describe(text));
        }
    )*};
}

fixture_tests!(
    deployment_with_full_metadata,
    // A GitHub workflow, a scalar document, an empty document and a sequence.
    documents_that_are_not_objects_are_skipped,
    either_key_alone_makes_an_object,
    invalid_api_versions_and_kinds_have_no_gvk,
    list_is_expanded_into_its_items,
);

#[test]
fn core_group_object_in_flow_style() {
    insta::assert_snapshot!(describe(
        "{apiVersion: v1, kind: ConfigMap, metadata: {name: é}}\n"
    ));
}

#[test]
fn duplicate_keys_use_the_last_value() {
    insta::assert_snapshot!(describe("kind: Pod\nkind: Service\napiVersion: v1\n"));
}

#[test]
fn builtin_kind_checks_the_group_of_a_usable_api_version() {
    let statefulset = Some(BuiltinKind::StatefulSet);
    let cases = [
        ("{apiVersion: v1, kind: Pod}", Some(BuiltinKind::Pod)),
        ("{apiVersion: apps/v1, kind: StatefulSet}", statefulset),
        ("{apiVersion: apps/v1beta2, kind: StatefulSet}", statefulset),
        ("{kind: StatefulSet}", statefulset),
        ("{apiVersion: a/b/c, kind: StatefulSet}", statefulset),
        ("{apiVersion: [apps/v1], kind: StatefulSet}", statefulset),
        ("{apiVersion: example.com/v1, kind: StatefulSet}", None),
        ("{apiVersion: v1, kind: StatefulSet}", None),
        ("{apiVersion: extensions/v1beta1, kind: Deployment}", None),
        ("{apiVersion: v1, kind: pod}", None),
        ("{apiVersion: v1, kind: [Pod]}", None),
        ("{apiVersion: example.com/v1, kind: Certificate}", None),
    ];
    for (text, expected) in cases {
        let parse = parse(text);
        assert_eq!(
            objects(&parse.documents[0])[0].builtin_kind(),
            expected,
            "{text}"
        );
    }
}
