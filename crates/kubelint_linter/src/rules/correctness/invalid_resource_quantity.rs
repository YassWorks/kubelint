//! KL1007 `invalid-resource-quantity`.

use kubelint_diagnostics::Severity;
use kubelint_k8s::{Object, Quantity, QuantityError};
use kubelint_yaml::{Node, NodeKind, ResolvedScalar};

use crate::{Category, ObjectContext, Rule, RuleMeta};

/// Reports a resource quantity that Kubernetes cannot parse, such as `512mb` or
/// `0.5cores`.
///
/// Checked places: container and pod-level `resources.requests`/`limits`, pod
/// `overhead`, `emptyDir.sizeLimit`, `PersistentVolumeClaim` resources (also in a
/// StatefulSet's `volumeClaimTemplates`), `ResourceQuota` `spec.hard` and `LimitRange`
/// limits. YAML numbers (`cpu: 0.5`, `memory: 1e9`) are valid quantities, and so is
/// null (Kubernetes reads it as zero). Mappings and sequences are schema problems and
/// are left to schema validation.
pub struct InvalidResourceQuantity;

static META: RuleMeta = RuleMeta {
    code: "KL1007",
    name: "invalid-resource-quantity",
    category: Category::Correctness,
    default_severity: Severity::Error,
    default_enabled: true,
    summary: "A resource quantity that Kubernetes cannot parse.",
    explanation: r"## What it does

Reports a CPU, memory or storage amount that is not a valid Kubernetes quantity, in
container and pod resources, pod overhead, `emptyDir.sizeLimit`, persistent volume
claims, resource quotas and limit ranges.

## Why it matters

Kubernetes writes amounts as a number with an optional, case-sensitive suffix: `Ki`,
`Mi`, `Gi`, `Ti`, `Pi`, `Ei` (powers of 1024), `n`, `u`, `m`, `k`, `M`, `G`, `T`, `P`,
`E` (powers of 1000) or an exponent such as `e3`. Units from other tools, like `512mb`,
`1GB`, `1K` or `0.5cores`, are rejected by the API server and the whole object fails to
apply. Note that `m` means milli: `memory: 512m` is half a byte, not 512 megabytes.

CPU is counted in cores without a unit: `0.5` and `500m` both mean half a core.

## Example

```yaml
resources:
  requests:
    cpu: 0.5cores
    memory: 512mb
```

Use instead:

```yaml
resources:
  requests:
    cpu: 500m
    memory: 512Mi
```
",
};

/// The `LimitRange` item fields that hold resource lists.
const LIMIT_RANGE_FIELDS: [&str; 5] = [
    "max",
    "min",
    "default",
    "defaultRequest",
    "maxLimitRequestRatio",
];

impl Rule for InvalidResourceQuantity {
    fn meta(&self) -> &'static RuleMeta {
        &META
    }

    fn check_object(&self, cx: &mut ObjectContext<'_>) {
        let object = cx.object();
        let mut lists: Vec<&Node> = Vec::new();
        let mut single: Vec<(&Node, &str)> = Vec::new();
        if let Some(pod) = object.pod_spec() {
            for container in pod.containers() {
                push_requirements(&mut lists, container.resources());
            }
            push_requirements(&mut lists, pod.node().get("resources"));
            lists.extend(pod.node().get("overhead"));
            for volume in pod.node().get("volumes").into_iter().flat_map(Node::items) {
                if let Some(size) = volume.get_path(&["emptyDir", "sizeLimit"]) {
                    single.push((size, "sizeLimit"));
                }
            }
            if is_kind(object, "StatefulSet", "apps") {
                let templates = object.node().get_path(&["spec", "volumeClaimTemplates"]);
                for template in templates.into_iter().flat_map(Node::items) {
                    push_requirements(&mut lists, template.get_path(&["spec", "resources"]));
                }
            }
        } else if is_kind(object, "PersistentVolumeClaim", "") {
            push_requirements(&mut lists, object.node().get_path(&["spec", "resources"]));
        } else if is_kind(object, "ResourceQuota", "") {
            lists.extend(object.node().get_path(&["spec", "hard"]));
        } else if is_kind(object, "LimitRange", "") {
            let limits = object.node().get_path(&["spec", "limits"]);
            for item in limits.into_iter().flat_map(Node::items) {
                lists.extend(
                    LIMIT_RANGE_FIELDS
                        .iter()
                        .filter_map(|field| item.get(field)),
                );
            }
        }
        for list in lists {
            for entry in list.entries() {
                if let Some(resource) = entry.key.as_str() {
                    single.push((&entry.value, resource));
                }
            }
        }
        for (value, resource) in single {
            check_value(cx, value, resource);
        }
    }
}

/// Adds the `requests` and `limits` lists of a `resources` block.
fn push_requirements<'a>(lists: &mut Vec<&'a Node>, resources: Option<&'a Node>) {
    let Some(resources) = resources else {
        return;
    };
    lists.extend(resources.get("requests"));
    lists.extend(resources.get("limits"));
}

/// Whether `object` is a `kind` of API group `group`. Without a usable `apiVersion` the
/// kind alone decides, as for workloads.
fn is_kind(object: Object<'_>, kind: &str, group: &str) -> bool {
    object.kind().and_then(Node::as_str) == Some(kind)
        && object.group_version().is_none_or(|gv| gv.group == group)
}

fn check_value(cx: &mut ObjectContext<'_>, value: &Node, resource: &str) {
    let NodeKind::Scalar(scalar) = &value.kind else {
        return;
    };
    let text = match scalar.resolve_kubernetes() {
        ResolvedScalar::Null | ResolvedScalar::Int(_) | ResolvedScalar::Uint(_) => return,
        ResolvedScalar::Float(number) if number.is_finite() => return,
        ResolvedScalar::Float(_) => {
            let message = invalid(&scalar.value, resource, "it is not a finite number");
            let diagnostic = cx.diagnostic(value.range, message).with_help(GENERIC_HELP);
            cx.report(diagnostic);
            return;
        }
        ResolvedScalar::Bool(_) => {
            let message = invalid(&scalar.value, resource, "YAML reads it as a boolean");
            let diagnostic = cx.diagnostic(value.range, message).with_help(GENERIC_HELP);
            cx.report(diagnostic);
            return;
        }
        ResolvedScalar::String => scalar.value.as_str(),
    };
    // Kubernetes trims surrounding whitespace before parsing (`Quantity.UnmarshalJSON`).
    let Err(error) = Quantity::parse(text.trim()) else {
        return;
    };
    let help = match error {
        QuantityError::InvalidSuffix(suffix) => {
            let number = &text.trim()[..text.trim().len() - suffix.len()];
            suggest_suffix(number, suffix).unwrap_or_else(|| GENERIC_HELP.to_owned())
        }
        QuantityError::Empty | QuantityError::InvalidNumber => GENERIC_HELP.to_owned(),
    };
    let message = invalid(text, resource, &error.to_string());
    let diagnostic = cx.diagnostic(value.range, message).with_help(help);
    cx.report(diagnostic);
}

/// The diagnostic message: `invalid quantity `512mb` for `memory`: <reason>`.
fn invalid(text: &str, resource: &str, reason: &str) -> String {
    if text.trim().is_empty() {
        format!("invalid quantity for `{resource}`: {reason}")
    } else {
        format!("invalid quantity `{text}` for `{resource}`: {reason}")
    }
}

const GENERIC_HELP: &str = "write a number with an optional suffix, such as `500m`, `256Mi` or `1G` \
     (`kubelint rule KL1007` lists the suffixes)";

/// A help text for a number followed by a suffix from another tool, such as `mb`,
/// `GiB`, `K` or `cores`. `None` when the suffix is not a recognized mistake.
fn suggest_suffix(number: &str, suffix: &str) -> Option<String> {
    // `20 Gi`, `1 CPU`: no whitespace is allowed between the number and the suffix.
    let unspaced = suffix.trim_start();
    if unspaced.len() < suffix.len() {
        let joined = format!("{number}{unspaced}");
        if Quantity::parse(&joined).is_ok() {
            return Some(format!("remove the space: write `{joined}`"));
        }
        return suggest_suffix(number, unspaced);
    }
    let lower = suffix.to_ascii_lowercase();
    if matches!(
        lower.as_str(),
        "core" | "cores" | "cpu" | "cpus" | "vcpu" | "vcpus"
    ) {
        return Some(format!(
            "CPU is counted in cores without a unit: write `{number}`{}",
            millis(number).map_or_else(String::new, |m| format!(" or `{m}m`"))
        ));
    }
    if matches!(
        lower.as_str(),
        "mcore" | "mcores" | "millicore" | "millicores" | "mcpu"
    ) {
        return Some(format!(
            "millicores are written with `m`: write `{number}m`"
        ));
    }
    if matches!(lower.as_str(), "b" | "byte" | "bytes") {
        return Some(format!("bytes need no unit: write `{number}`"));
    }
    // `mb`, `MB`, `Mib`, `MiB`, `gi`, `K`: a size unit in the wrong spelling.
    let unit = lower.strip_suffix('b').unwrap_or(&lower);
    let (letter, binary_only) = match unit.strip_suffix('i') {
        Some(letter) => (letter, true),
        None => (unit, false),
    };
    let decimal = match letter {
        "k" => "k",
        "m" => "M",
        "g" => "G",
        "t" => "T",
        "p" => "P",
        "e" => "E",
        _ => return None,
    };
    let binary = format!("{}i", decimal.to_ascii_uppercase());
    Some(if binary_only {
        format!("suffixes are case-sensitive: write `{number}{binary}`")
    } else {
        format!("write `{number}{binary}` (powers of 1024) or `{number}{decimal}` (powers of 1000)")
    })
}

/// `number` in thousandths, when that is a whole number: `0.5` is `500`.
fn millis(number: &str) -> Option<String> {
    let (integer, fraction) = number.split_once('.').unwrap_or((number, ""));
    if fraction.len() > 3 || integer.starts_with(['+', '-']) {
        return None;
    }
    let digits = format!("{integer}{fraction:0<3}");
    let trimmed = digits.trim_start_matches('0');
    Some(if trimmed.is_empty() { "0" } else { trimmed }.to_owned())
}

#[cfg(test)]
mod tests {
    use super::{InvalidResourceQuantity, millis, suggest_suffix};

    #[test]
    fn fixture() {
        insta::assert_snapshot!(crate::testing::check_fixture(&InvalidResourceQuantity));
    }

    #[test]
    fn suffix_suggestions() {
        let cases = [
            (
                "512",
                "mb",
                Some("write `512Mi` (powers of 1024) or `512M` (powers of 1000)"),
            ),
            (
                "1",
                "GB",
                Some("write `1Gi` (powers of 1024) or `1G` (powers of 1000)"),
            ),
            (
                "1",
                "K",
                Some("write `1Ki` (powers of 1024) or `1k` (powers of 1000)"),
            ),
            ("2", "gi", Some("suffixes are case-sensitive: write `2Gi`")),
            ("2", "MiB", Some("suffixes are case-sensitive: write `2Mi`")),
            ("100", "bytes", Some("bytes need no unit: write `100`")),
            (
                "0.5",
                "cores",
                Some("CPU is counted in cores without a unit: write `0.5` or `500m`"),
            ),
            (
                "2",
                "CPU",
                Some("CPU is counted in cores without a unit: write `2` or `2000m`"),
            ),
            (
                "250",
                "mcores",
                Some("millicores are written with `m`: write `250m`"),
            ),
            ("20", " Gi", Some("remove the space: write `20Gi`")),
            (
                "1",
                " CPU",
                Some("CPU is counted in cores without a unit: write `1` or `1000m`"),
            ),
            ("1", "x", None),
            ("1", "ib", None),
        ];
        for (number, suffix, expected) in cases {
            assert_eq!(
                suggest_suffix(number, suffix).as_deref(),
                expected,
                "{number}{suffix}"
            );
        }
    }

    #[test]
    fn millis_of_numbers() {
        assert_eq!(millis("0.5").as_deref(), Some("500"));
        assert_eq!(millis(".25").as_deref(), Some("250"));
        assert_eq!(millis("1.").as_deref(), Some("1000"));
        assert_eq!(millis("0").as_deref(), Some("0"));
        assert_eq!(millis("0.0001"), None);
        assert_eq!(millis("-1"), None);
    }
}
