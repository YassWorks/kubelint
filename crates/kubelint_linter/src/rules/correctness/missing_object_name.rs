//! KL1003 `missing-object-name`.

use kubelint_diagnostics::Severity;
use kubelint_k8s::{BuiltinKind, Object};
use kubelint_source::TextRange;

use super::{entry_range, is_empty};
use crate::{Category, ObjectContext, Rule, RuleMeta};

/// Reports an object without a `metadata.name` (or `metadata.generateName`), or with an
/// empty one.
///
/// An object with no `metadata` key at all is reported only when its kind is a built-in
/// kind. Files such as `kustomization.yaml`, a kubeconfig (`kind: Config`) or a
/// `KubeletConfiguration` have `apiVersion` and `kind` but are configuration files, not
/// API objects, and never have a name.
pub struct MissingObjectName;

static META: RuleMeta = RuleMeta {
    code: "KL1003",
    name: "missing-object-name",
    category: Category::Correctness,
    default_severity: Severity::Error,
    default_enabled: true,
    summary: "Object has no `metadata.name`.",
    explanation: include_str!("missing_object_name.md"),
};

impl Rule for MissingObjectName {
    fn meta(&self) -> &'static RuleMeta {
        &META
    }

    fn check_object(&self, cx: &mut ObjectContext<'_>) {
        let object = cx.object();
        let Some((range, empty)) = problem(object) else {
            return;
        };
        let subject = match object.kind().and_then(|kind| kind.as_str()) {
            Some(kind) if !kind.trim().is_empty() => format!("`{kind}` object"),
            _ => "object".to_owned(),
        };
        let message = if empty {
            format!("{subject} has an empty `metadata.name`")
        } else {
            format!("{subject} has no `metadata.name`")
        };
        cx.report(
            cx.diagnostic(range, message)
                .with_help("add a name under `metadata`, such as `name: web`"),
        );
    }
}

/// Where to point and whether the name is present but empty, or `None` when the object
/// has a usable name or the rule cannot tell.
fn problem(object: Object<'_>) -> Option<(TextRange, bool)> {
    let Some(metadata) = object.node().get_entry("metadata") else {
        let kind = object.node().get_entry("kind")?;
        let is_builtin = kind.value.as_str().and_then(BuiltinKind::parse).is_some();
        return is_builtin.then(|| (entry_range(kind), false));
    };
    if is_empty(&metadata.value) {
        // `metadata:` with no value: point at the key, as there is nothing else.
        return Some((metadata.key.range, false));
    }
    // A string or a sequence is a schema problem.
    metadata.value.as_mapping()?;
    let usable = |key: &str| {
        metadata
            .value
            .get(key)
            .is_some_and(|value| !is_empty(value))
    };
    if usable("name") || usable("generateName") {
        return None;
    }
    match metadata.value.get_entry("name") {
        Some(name) => Some((entry_range(name), true)),
        None => Some((metadata.key.range, false)),
    }
}
