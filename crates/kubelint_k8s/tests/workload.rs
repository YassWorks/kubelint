//! Workload helpers over one fixture per workload kind, in
//! `resources/test/fixtures/workloads/`. Fixtures contain non-ASCII text so ranges in
//! snapshots are byte offsets.

use std::fmt::Write as _;

use kubelint_k8s::objects;
use kubelint_yaml::{Node, parse};

/// For every object: its kind, its pod spec range and every container with the
/// accessors that found something.
fn describe(text: &str) -> String {
    let parse = parse(text);
    assert!(parse.errors.is_empty(), "{:?}", parse.errors);
    let mut out = String::new();
    for object in parse.documents.iter().flat_map(objects) {
        let kind = object.kind().and_then(Node::as_str).unwrap_or("?");
        let Some(pod) = object.pod_spec() else {
            let _ = writeln!(out, "{kind}: no pod spec");
            continue;
        };
        let _ = writeln!(out, "{kind}: pod spec {}", fmt_range(pod.node()));
        if let Some(sc) = pod.security_context() {
            let _ = writeln!(out, "  pod securityContext {}", fmt_range(sc));
        }
        for c in pod.containers() {
            let name = c.name().and_then(Node::as_str).unwrap_or("?");
            let _ = writeln!(
                out,
                "  {} {name:?} {}",
                c.kind().field(),
                fmt_range(c.node())
            );
            let fields = [
                ("image", c.image()),
                ("securityContext", c.security_context()),
                ("resources", c.resources()),
                ("env", c.env()),
                ("livenessProbe", c.liveness_probe()),
                ("readinessProbe", c.readiness_probe()),
                ("startupProbe", c.startup_probe()),
            ];
            for (field, node) in fields {
                if let Some(node) = node {
                    let _ = writeln!(out, "    {field} {}", fmt_range(node));
                }
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

macro_rules! fixture_tests {
    ($($name:ident),* $(,)?) => {$(
        #[test]
        fn $name() {
            let text = include_str!(concat!(
                "../resources/test/fixtures/workloads/",
                stringify!($name),
                ".yaml"
            ));
            insta::assert_snapshot!(describe(text));
        }
    )*};
}

fixture_tests!(
    pod,
    deployment,
    replicaset,
    daemonset,
    statefulset,
    replicationcontroller,
    job,
    cronjob,
    not_workloads,
);
