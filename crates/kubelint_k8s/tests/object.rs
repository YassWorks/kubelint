//! `Object` views over parsed manifests. Inputs contain non-ASCII text so the ranges in
//! snapshots are byte offsets.

use std::fmt::Write as _;

use kubelint_k8s::objects;
use kubelint_yaml::{Node, parse};

/// One block per object: its range and each accessor's value with its range.
fn describe(text: &str) -> String {
    let parse = parse(text);
    assert!(parse.errors.is_empty(), "{:?}", parse.errors);
    let mut out = String::new();
    for (i, doc) in parse.documents.iter().enumerate() {
        for object in objects(doc) {
            let _ = writeln!(out, "doc {i} object {}", fmt_range(object.node()));
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
                    let _ = writeln!(out, "  {field} {shown} {}", fmt_range(node));
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

fn fmt_range(node: &Node) -> String {
    format!(
        "{}..{}",
        u32::from(node.range.start()),
        u32::from(node.range.end())
    )
}

#[test]
fn deployment_with_full_metadata() {
    insta::assert_snapshot!(describe(
        "# é\napiVersion: apps/v1\nkind: Deployment\nmetadata:\n  name: wéb\n  namespace: prod\n  labels:\n    app: web\n  annotations:\n    note: \"ü\"\n"
    ));
}

#[test]
fn core_group_object_in_flow_style() {
    insta::assert_snapshot!(describe(
        "{apiVersion: v1, kind: ConfigMap, metadata: {name: é}}\n"
    ));
}

#[test]
fn documents_that_are_not_objects_are_skipped() {
    // A GitHub workflow, a scalar document, an empty document and a sequence.
    insta::assert_snapshot!(describe(
        "name: ci\non: push\n---\njust text é\n---\n---\n- kind: Pod\n"
    ));
}

#[test]
fn either_key_alone_makes_an_object() {
    insta::assert_snapshot!(describe(
        "kind: Pod\n---\napiVersion: v1\n---\nkind:\nmetadata: {name: é}\n"
    ));
}

#[test]
fn invalid_api_versions_and_kinds_have_no_gvk() {
    insta::assert_snapshot!(describe(
        "apiVersion: a/b/c\nkind: Pod\n---\napiVersion: v1\nkind: [Pod]\n---\napiVersion: 1\nkind: \"\"\n"
    ));
}

#[test]
fn list_is_expanded_into_its_items() {
    insta::assert_snapshot!(describe(
        "apiVersion: v1\nkind: List\nitems:\n- apiVersion: v1\n  kind: Service\n  metadata: {name: é}\n- not an object\n- metadata: {name: no-kind}\n---\nkind: List\nitems: nope\n"
    ));
}

#[test]
fn duplicate_keys_use_the_last_value() {
    insta::assert_snapshot!(describe("kind: Pod\nkind: Service\napiVersion: v1\n"));
}
