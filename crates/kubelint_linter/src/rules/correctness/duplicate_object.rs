//! KL1004 `duplicate-object`.

use std::collections::HashMap;

use kubelint_diagnostics::{Label, Severity};
use kubelint_k8s::Object;
use kubelint_source::TextRange;

use super::entry_range;
use crate::{Category, Rule, RuleMeta, RunContext};

/// Reports an object defined more than once across the linted files: the same API
/// group, kind, namespace and name.
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
    summary: "The same object is defined more than once.",
    explanation: r"## What it does

Reports an object whose API group, `kind`, `metadata.namespace` and `metadata.name` are
the same as those of an object defined earlier, in the same file or in another linted
file.

## Why it matters

Kubernetes identifies an object by its kind, namespace and name. When two manifests
define the same object, `kubectl apply` applies both, one after the other, and the last
one silently wins. Which one that is depends on file order, so a change to the first
definition may never reach the cluster. This usually comes from copying a manifest and
forgetting to rename it.

Objects without a namespace are only compared with other objects without a namespace,
because their namespace is chosen when they are applied.

## Example

```yaml
apiVersion: v1
kind: Service
metadata:
  name: web
---
apiVersion: v1
kind: Service
metadata:
  name: web
```

Use instead:

```yaml
apiVersion: v1
kind: Service
metadata:
  name: web
---
apiVersion: v1
kind: Service
metadata:
  name: web-internal
```
",
};

/// What makes two objects the same object.
#[derive(PartialEq, Eq, Hash)]
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
            let Some(&(first_file, first_range)) = first.get(&identity) else {
                first.insert(identity, (lint.file, range));
                continue;
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
            let label = Label::new(first_file, first_range).with_message("first defined here");
            let diagnostic = cx
                .diagnostic(lint.file, range, message)
                .with_secondary(label)
                .with_help(
                    "rename one of them or remove the copy; when applied, the last one \
                     silently replaces the first",
                );
            cx.report(diagnostic);
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

#[cfg(test)]
mod tests {
    use kubelint_source::{FileId, SourceFile};

    use super::DuplicateObject;
    use crate::{EnabledRule, Linter};

    #[test]
    fn fixture() {
        insta::assert_snapshot!(crate::testing::check_fixture(&DuplicateObject));
    }

    #[test]
    fn points_back_at_another_file() {
        let manifest = "apiVersion: v1\nkind: Service\nmetadata:\n  name: web\n";
        let files = [
            SourceFile::new("a.yaml", manifest),
            SourceFile::new("b.yaml", format!("# copy\n{manifest}")),
        ];
        let linter = Linter::new(vec![EnabledRule::new(&DuplicateObject)]);
        let result = linter.lint(&files);
        let [diagnostic] = result.diagnostics.as_slice() else {
            panic!("expected one finding: {:?}", result.diagnostics);
        };
        assert_eq!(diagnostic.primary.file, FileId(1));
        assert_eq!(
            diagnostic.message,
            "`Service` `web` is defined more than once"
        );
        let [first] = diagnostic.secondary.as_slice() else {
            panic!("expected one secondary label");
        };
        assert_eq!(first.file, FileId(0));
        assert_eq!(&manifest[first.range], "name: web");
    }

    #[test]
    fn suppression_in_the_copy_silences_it() {
        let manifest = "apiVersion: v1\nkind: Service\nmetadata:\n  name: web\n";
        let files = [
            SourceFile::new("a.yaml", manifest),
            SourceFile::new(
                "b.yaml",
                format!("# kubelint-disable-file duplicate-object\n{manifest}"),
            ),
        ];
        let linter = Linter::new(vec![EnabledRule::new(&DuplicateObject)]);
        assert!(linter.lint(&files).diagnostics.is_empty());
    }
}
