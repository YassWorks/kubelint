//! KL1007 `invalid-resource-quantity`.

use kubelint_diagnostics::Severity;
use kubelint_k8s::{BuiltinKind, Quantity, QuantityError};
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
    explanation: include_str!("invalid_resource_quantity.md"),
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
        let kind = object.builtin_kind();
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
            if kind == Some(BuiltinKind::StatefulSet) {
                let templates = object.node().get_path(&["spec", "volumeClaimTemplates"]);
                for template in templates.into_iter().flat_map(Node::items) {
                    push_requirements(&mut lists, template.get_path(&["spec", "resources"]));
                }
            }
        } else {
            match kind {
                Some(BuiltinKind::PersistentVolumeClaim) => {
                    push_requirements(&mut lists, object.node().get_path(&["spec", "resources"]));
                }
                Some(BuiltinKind::ResourceQuota) => {
                    lists.extend(object.node().get_path(&["spec", "hard"]));
                }
                Some(BuiltinKind::LimitRange) => {
                    let limits = object.node().get_path(&["spec", "limits"]);
                    for item in limits.into_iter().flat_map(Node::items) {
                        lists.extend(
                            LIMIT_RANGE_FIELDS
                                .iter()
                                .filter_map(|field| item.get(field)),
                        );
                    }
                }
                _ => {}
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

fn check_value(cx: &mut ObjectContext<'_>, value: &Node, resource: &str) {
    let NodeKind::Scalar(scalar) = &value.kind else {
        return;
    };
    let text = scalar.value.as_str();
    let (reason, help) = match scalar.resolve_kubernetes() {
        ResolvedScalar::Null | ResolvedScalar::Int(_) | ResolvedScalar::Uint(_) => return,
        ResolvedScalar::Float(number) if number.is_finite() => return,
        ResolvedScalar::Float(_) => ("it is not a finite number".to_owned(), None),
        ResolvedScalar::Bool(_) => ("YAML reads it as a boolean".to_owned(), None),
        ResolvedScalar::String => {
            // Kubernetes trims surrounding whitespace before parsing (`Quantity.UnmarshalJSON`).
            let trimmed = text.trim();
            let Err(error) = Quantity::parse(trimmed) else {
                return;
            };
            let help = match error {
                QuantityError::InvalidSuffix(suffix) => {
                    suggest_suffix(&trimmed[..trimmed.len() - suffix.len()], suffix)
                }
                QuantityError::Empty | QuantityError::InvalidNumber => None,
            };
            (error.to_string(), help)
        }
    };
    cx.report(
        cx.diagnostic(value.range, invalid(text, resource, &reason))
            .with_help(help.as_deref().unwrap_or(GENERIC_HELP)),
    );
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
mod tests;
