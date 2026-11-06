//! KL1005 `selector-mismatch`.

use kubelint_diagnostics::Severity;
use kubelint_source::TextRange;
use kubelint_yaml::{Entry, Node};

use super::entry_range;
use crate::{Category, ObjectContext, Rule, RuleMeta};

/// Reports a workload whose `spec.selector` does not select its own pod template, one
/// finding per selector requirement the template's labels fail.
///
/// Covers Deployment, ReplicaSet, DaemonSet, StatefulSet and Job (a label selector with
/// `matchLabels` and `matchExpressions`) and ReplicationController (a plain map). A
/// missing selector, an unknown operator or a value of the wrong type is a schema
/// problem, and the requirement is skipped.
pub struct SelectorMismatch;

static META: RuleMeta = RuleMeta {
    code: "KL1005",
    name: "selector-mismatch",
    category: Category::Correctness,
    default_severity: Severity::Error,
    default_enabled: true,
    summary: "A workload's selector does not match its pod template labels.",
    explanation: r#"## What it does

Reports a Deployment, ReplicaSet, DaemonSet, StatefulSet, Job or ReplicationController
whose `spec.selector` does not match the labels in `spec.template.metadata.labels`.

## Why it matters

A workload finds the pods it owns with its selector. The pods it creates get the labels
of its pod template. If the template's labels do not satisfy the selector, the workload
could never see its own pods, so the API server rejects it with
"`selector` does not match template `labels`".

The selector of a Deployment, ReplicaSet, DaemonSet or StatefulSet cannot be changed
after it is created, so it is worth getting right before the first apply.

## Example

```yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: web
spec:
  selector:
    matchLabels:
      app: web
  template:
    metadata:
      labels:
        app: frontend
```

Use instead:

```yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: web
spec:
  selector:
    matchLabels:
      app: web
  template:
    metadata:
      labels:
        app: web
```
"#,
};

const HELP: &str = "make `spec.selector` and `spec.template.metadata.labels` agree, usually by adding \
     the label to the template";

impl Rule for SelectorMismatch {
    fn meta(&self) -> &'static RuleMeta {
        &META
    }

    fn check_object(&self, cx: &mut ObjectContext<'_>) {
        let object = cx.object();
        let Some(kind) = object.kind().and_then(Node::as_str) else {
            return;
        };
        let plain_map = match kind {
            "ReplicationController" => true,
            "Deployment" | "ReplicaSet" | "DaemonSet" | "StatefulSet" | "Job" => false,
            _ => return,
        };
        // `pod_spec` checks that the kind is in the right API group.
        if object.pod_spec().is_none() {
            return;
        }
        let Some(spec) = object.node().get("spec") else {
            return;
        };
        let (Some(selector), Some(template)) = (spec.get("selector"), spec.get_entry("template"))
        else {
            return;
        };
        let template = Template::new(template);
        let match_labels = if plain_map {
            Some(selector)
        } else {
            selector.get("matchLabels")
        };
        let mut findings = Vec::new();
        for entry in match_labels.into_iter().flat_map(Node::entries) {
            let (Some(key), Some(value)) = (entry.key_str(), entry.value.as_str()) else {
                continue;
            };
            if template.value(key) != Some(value) {
                let requirement = format!("`{key}: {value}`");
                findings.push((entry_range(entry), requirement, key));
            }
        }
        let expressions = selector.get("matchExpressions").filter(|_| !plain_map);
        for expression in expressions.into_iter().flat_map(Node::items) {
            let Some(requirement) = Requirement::parse(expression) else {
                continue;
            };
            if !requirement.matches(template.value(requirement.key)) {
                findings.push((expression.range, requirement.to_string(), requirement.key));
            }
        }
        for (range, requirement, key) in findings {
            let (message, label) = match template.entry(key) {
                Some(entry) => (
                    format!(
                        "`{kind}` selector requires {requirement}, but the pod template has \
                         `{key}: {}`",
                        entry.value.as_str().unwrap_or_default()
                    ),
                    cx.label(entry_range(entry))
                        .with_message("pod template label"),
                ),
                None => (
                    format!(
                        "`{kind}` selector requires {requirement}, but the pod template has \
                         no `{key}` label"
                    ),
                    cx.label(template.anchor)
                        .with_message(template.anchor_message()),
                ),
            };
            let diagnostic = cx
                .diagnostic(range, message)
                .with_secondary(label)
                .with_help(HELP);
            cx.report(diagnostic);
        }
    }
}

/// The pod template's labels, and where to point when one is missing.
struct Template<'a> {
    labels: Option<&'a Node>,
    /// The `labels` key, or the closest existing parent key.
    anchor: TextRange,
}

impl<'a> Template<'a> {
    fn new(template: &'a Entry) -> Self {
        let metadata = template.value.get_entry("metadata");
        let labels = metadata.and_then(|metadata| metadata.value.get_entry("labels"));
        let anchor = labels
            .or(metadata)
            .map_or(template.key.range, |entry| entry.key.range);
        Self {
            labels: labels.map(|entry| &entry.value),
            anchor,
        }
    }

    fn anchor_message(&self) -> &'static str {
        if self.labels.is_some() {
            "pod template labels"
        } else {
            "pod template has no labels"
        }
    }

    fn entry(&self, key: &str) -> Option<&'a Entry> {
        self.labels?.get_entry(key)
    }

    fn value(&self, key: &str) -> Option<&'a str> {
        self.entry(key)?.value.as_str()
    }
}

/// One `matchExpressions` item.
struct Requirement<'a> {
    key: &'a str,
    operator: Operator,
    values: Vec<&'a str>,
}

#[derive(Clone, Copy)]
enum Operator {
    In,
    NotIn,
    Exists,
    DoesNotExist,
}

impl<'a> Requirement<'a> {
    /// `None` when the expression is malformed: the API server reports those.
    fn parse(node: &'a Node) -> Option<Self> {
        let key = node.get("key")?.as_str()?;
        let operator = match node.get("operator")?.as_str()? {
            "In" => Operator::In,
            "NotIn" => Operator::NotIn,
            "Exists" => Operator::Exists,
            "DoesNotExist" => Operator::DoesNotExist,
            _ => return None,
        };
        let values = node
            .get("values")
            .into_iter()
            .flat_map(Node::items)
            .map(Node::as_str)
            .collect::<Option<Vec<_>>>()?;
        Some(Self {
            key,
            operator,
            values,
        })
    }

    /// Whether a pod with `label` as the value of the key (or no such label) is selected.
    fn matches(&self, label: Option<&str>) -> bool {
        match self.operator {
            Operator::In => label.is_some_and(|value| self.values.contains(&value)),
            Operator::NotIn => label.is_none_or(|value| !self.values.contains(&value)),
            Operator::Exists => label.is_some(),
            Operator::DoesNotExist => label.is_none(),
        }
    }
}

impl std::fmt::Display for Requirement<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let key = self.key;
        let values = self.values.join(", ");
        match self.operator {
            Operator::In => write!(f, "`{key}` in [{values}]"),
            Operator::NotIn => write!(f, "`{key}` not in [{values}]"),
            Operator::Exists => write!(f, "the `{key}` label"),
            Operator::DoesNotExist => write!(f, "no `{key}` label"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::SelectorMismatch;

    #[test]
    fn fixture() {
        insta::assert_snapshot!(crate::testing::check_fixture(&SelectorMismatch));
    }
}
