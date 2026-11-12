//! KL1004 `duplicate-object`.

use std::collections::HashMap;
use std::collections::hash_map::Entry;

use kubelint_diagnostics::{Label, Severity};
use kubelint_k8s::Object;
use kubelint_source::TextRange;

use super::entry_range;
use crate::{Category, Rule, RuleMeta, RunContext};

/// Reports an object defined more than once in one file: the same API group, kind,
/// namespace and name.
///
/// The version is not part of the identity, because `apps/v1` and `apps/v1beta2` serve
/// the same objects. A missing namespace is kept apart from every written one, including
/// `default`: where such an object lands depends on how it is applied
/// (`kubectl apply -n`). Objects without a usable `apiVersion`, `kind` or `name` are
/// left to the rules that report those.
pub struct DuplicateObject;

static META: RuleMeta = RuleMeta {
    code: "KL1004",
    name: "duplicate-object",
    category: Category::Correctness,
    default_severity: Severity::Error,
    default_enabled: true,
    summary: "The same object is defined more than once in one file.",
    explanation: include_str!("duplicate_object.md"),
};

/// What makes two objects the same object.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
struct Identity<'a> {
    group: &'a str,
    kind: &'a str,
    namespace: Option<&'a str>,
    name: &'a str,
}

impl Rule for DuplicateObject {
    fn meta(&self) -> &'static RuleMeta {
        &META
    }

    fn check_all(&self, cx: &mut RunContext<'_>) {
        let mut first = HashMap::new();
        for lint in cx.objects() {
            let Some((identity, range)) = identity(lint.object) else {
                continue;
            };
            let first_range = match first.entry((lint.file, identity)) {
                Entry::Occupied(entry) => *entry.get(),
                Entry::Vacant(entry) => {
                    entry.insert(range);
                    continue;
                }
            };
            let message = match identity.namespace {
                Some(namespace) => format!(
                    "`{}` `{}` in namespace `{namespace}` is defined more than once",
                    identity.kind, identity.name
                ),
                None => format!(
                    "`{}` `{}` is defined more than once",
                    identity.kind, identity.name
                ),
            };
            cx.report(
                cx.diagnostic(lint.file, range, message)
                    .with_secondary(
                        Label::new(lint.file, first_range).with_message("first defined here"),
                    )
                    .with_help(
                        "rename one of them or remove the copy; when applied, the last one \
                         silently replaces the first",
                    ),
            );
        }
    }
}

/// The identity of `object` and the range of its `name: ...` entry, or `None` when it
/// has no usable `apiVersion`, `kind` or `name`.
fn identity(object: Object<'_>) -> Option<(Identity<'_>, TextRange)> {
    let gvk = object.gvk()?;
    let metadata = object.metadata()?;
    let name_entry = metadata.get_entry("name")?;
    let name = name_entry
        .value
        .as_str()
        .filter(|name| !name.trim().is_empty())?;
    let namespace = object
        .namespace()
        .and_then(|namespace| namespace.as_str())
        .filter(|namespace| !namespace.trim().is_empty());
    let identity = Identity {
        group: gvk.group,
        kind: gvk.kind,
        namespace,
        name,
    };
    Some((identity, entry_range(name_entry)))
}
