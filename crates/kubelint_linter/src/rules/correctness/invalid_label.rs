//! KL1010 `invalid-label`.

use kubelint_diagnostics::Severity;
use kubelint_k8s::{
    BuiltinKind, MAX_LABEL_LENGTH, Object, ValidationErrorKind, is_qualified_name,
    is_valid_label_value,
};
use kubelint_yaml::{Entry, Node, NodeKind, ResolvedScalar};

use super::invalid_object_name::subdomain_suggestion;
use super::{has_placeholder, shown};
use crate::{Category, ObjectContext, Rule, RuleMeta};

/// Reports a label key or value that breaks Kubernetes' label syntax, and a label value
/// that YAML reads as a number or boolean.
///
/// Checked maps: the object's `metadata.labels`, the pod template's (and a CronJob's
/// job template's) `metadata.labels`, a workload's `spec.selector.matchLabels` (a
/// ReplicationController's plain `spec.selector`) and a Service's `spec.selector`.
/// Selector `matchExpressions` are left alone. A null value is read as an empty
/// string, which is valid.
pub struct InvalidLabel;

static META: RuleMeta = RuleMeta {
    code: "KL1010",
    name: "invalid-label",
    category: Category::Correctness,
    default_severity: Severity::Error,
    default_enabled: true,
    summary: "A label key or value that breaks Kubernetes' label syntax.",
    explanation: include_str!("invalid_label.md"),
};

impl Rule for InvalidLabel {
    fn meta(&self) -> &'static RuleMeta {
        &META
    }

    fn check_object(&self, cx: &mut ObjectContext<'_>) {
        for (map, what) in label_maps(cx.object()) {
            for entry in map.entries() {
                check_entry(cx, entry, what);
            }
        }
    }
}

/// The label maps of `object`, each with how messages call it.
fn label_maps(object: Object<'_>) -> Vec<(&Node, &'static str)> {
    let mut maps: Vec<(&Node, &'static str)> = Vec::new();
    maps.extend(object.labels().map(|labels| (labels, "label")));
    let spec = object.node().get("spec");
    if object.pod_spec().is_some()
        && let Some(spec) = spec
    {
        // `pod_spec` has checked the API group.
        let kind = object
            .kind()
            .and_then(Node::as_str)
            .and_then(BuiltinKind::parse);
        let templates: &[&[&str]] = match kind {
            Some(BuiltinKind::CronJob) => &[
                &["jobTemplate", "metadata", "labels"],
                &["jobTemplate", "spec", "template", "metadata", "labels"],
            ],
            Some(BuiltinKind::Pod) => &[],
            _ => &[&["template", "metadata", "labels"]],
        };
        for path in templates {
            maps.extend(spec.get_path(path).map(|labels| (labels, "label")));
        }
        let selector = match kind {
            Some(BuiltinKind::ReplicationController) => spec.get("selector"),
            _ => spec.get_path(&["selector", "matchLabels"]),
        };
        maps.extend(selector.map(|labels| (labels, "selector label")));
    } else if object.builtin_kind() == Some(BuiltinKind::Service)
        && let Some(selector) = spec.and_then(|spec| spec.get("selector"))
    {
        maps.push((selector, "selector label"));
    }
    maps
}

fn check_entry(cx: &mut ObjectContext<'_>, entry: &Entry, what: &str) {
    // A key that is not a string is a schema problem.
    let Some(key) = entry.key_str() else {
        return;
    };
    if !has_placeholder(key)
        && let Err(error) = is_qualified_name(key)
    {
        let message = format!("{what} key `{}` is invalid: {error}", shown(key));
        // Only a wrong character has one obvious fix. A key that is too long or has two
        // slashes needs the author to choose what to drop.
        let example = match error.kind {
            ValidationErrorKind::Format(_) => key_suggestion(key),
            _ => None,
        };
        let help = format!(
            "write the key as an optional lowercase DNS prefix and `/`, then up to \
             {MAX_LABEL_LENGTH} letters, digits, `-`, `_` and `.`, for example `{}`",
            example.as_deref().unwrap_or("app.kubernetes.io/name")
        );
        cx.report(cx.diagnostic(entry.key.range, message).with_help(help));
    }
    let NodeKind::Scalar(scalar) = &entry.value.kind else {
        return;
    };
    let type_name = match scalar.resolve_kubernetes() {
        ResolvedScalar::Null => return,
        ResolvedScalar::String => None,
        ResolvedScalar::Bool(_) => Some("a boolean"),
        ResolvedScalar::Int(_) | ResolvedScalar::Uint(_) | ResolvedScalar::Float(_) => {
            Some("a number")
        }
    };
    let value = scalar.value.as_str();
    if let Some(type_name) = type_name {
        let message = format!(
            "{what} `{}` has a value that YAML reads as {type_name}, not a string",
            shown(key)
        );
        cx.report(
            cx.diagnostic(entry.value.range, message)
                .with_help(format!("quote the value: `{key}: \"{value}\"`")),
        );
        return;
    }
    if !has_placeholder(value)
        && let Err(error) = is_valid_label_value(value)
    {
        let message = format!(
            "{what} value `{}` of `{}` is invalid: {error}",
            shown(value),
            shown(key)
        );
        let help = match suggestion(value) {
            _ if value.len() > MAX_LABEL_LENGTH => format!(
                "shorten the value to at most {MAX_LABEL_LENGTH} characters, or move it to \
                 an annotation"
            ),
            Some(fixed) => format!(
                "use letters, digits, `-`, `_` and `.`, for example `{fixed}`; free text \
                 belongs in an annotation"
            ),
            None => "use letters, digits, `-`, `_` and `.`; free text belongs in an \
                     annotation"
                .to_owned(),
        };
        cx.report(cx.diagnostic(entry.value.range, message).with_help(help));
    }
}

/// A valid label key close to `key`: the prefix fixed like a DNS subdomain object name
/// (lowercased, at most 253 characters) and the name part like a label value (at most
/// 63). `None` when either part has nothing usable left or is not ASCII.
fn key_suggestion(key: &str) -> Option<String> {
    match key.split_once('/') {
        Some((prefix, name)) => Some(format!(
            "{}/{}",
            subdomain_suggestion(prefix)?,
            suggestion(name)?
        )),
        None => suggestion(key),
    }
}

/// A valid label value close to `value`: runs of other characters become one `-`, the
/// ends are trimmed and it is shortened to 63 characters. `None` for non-ASCII values
/// and when nothing usable is left.
fn suggestion(value: &str) -> Option<String> {
    if !value.is_ascii() {
        return None;
    }
    let allowed = |c: char| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.');
    let mut out = String::new();
    for c in value.chars() {
        if allowed(c) {
            out.push(c);
        } else if !out.ends_with('-') {
            out.push('-');
        }
    }
    let not_alnum = |c: char| !c.is_ascii_alphanumeric();
    let trimmed = out.trim_matches(not_alnum);
    let shortened = trimmed
        .get(..MAX_LABEL_LENGTH.min(trimmed.len()))?
        .trim_end_matches(not_alnum);
    (!shortened.is_empty() && is_valid_label_value(shortened).is_ok()).then(|| shortened.to_owned())
}

#[cfg(test)]
mod tests;
