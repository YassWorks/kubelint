//! KL1008 `invalid-object-name`.

use kubelint_diagnostics::Severity;
use kubelint_k8s::{
    BuiltinKind, MAX_LABEL_LENGTH, MAX_SUBDOMAIN_LENGTH, ValidationError, is_dns1035_label,
    is_dns1123_label, is_dns1123_subdomain,
};
use kubelint_yaml::{Node, NodeKind, ResolvedScalar};

use super::shown;
use crate::{Category, ObjectContext, Rule, RuleMeta};

/// Reports a `metadata.name` that the API server would reject for the object's kind,
/// and a `metadata.namespace` that is not a valid namespace name.
///
/// Names are checked only for built-in kinds ([`BuiltinKind`], in their own API group),
/// because a custom resource's name rules are up to its definition. Most kinds need a
/// DNS-1123 subdomain, `Namespace` a DNS-1123 label, `Service` a DNS-1035 label, and the
/// RBAC kinds only a valid path segment (so `system:controller:foo` is fine). Values that
/// are not strings are schema problems and are skipped, as are empty names (KL1003).
pub struct InvalidObjectName;

static META: RuleMeta = RuleMeta {
    code: "KL1008",
    name: "invalid-object-name",
    category: Category::Correctness,
    default_severity: Severity::Error,
    default_enabled: true,
    summary: "An object name or namespace that Kubernetes would reject.",
    explanation: include_str!("invalid_object_name.md"),
};

/// The syntax a kind's names must follow.
#[derive(Debug, Clone, Copy)]
enum NameFormat {
    Subdomain,
    Label,
    Dns1035,
    PathSegment,
}

impl NameFormat {
    /// The longest name allowed, in bytes. Path segments have no limit of their own.
    fn max_len(self) -> usize {
        match self {
            Self::Subdomain => MAX_SUBDOMAIN_LENGTH,
            Self::Label | Self::Dns1035 => MAX_LABEL_LENGTH,
            Self::PathSegment => usize::MAX,
        }
    }
}

impl Rule for InvalidObjectName {
    fn meta(&self) -> &'static RuleMeta {
        &META
    }

    fn check_object(&self, cx: &mut ObjectContext<'_>) {
        let object = cx.object();
        if let Some(namespace) = object.namespace()
            && let Some(text) = string_value(namespace)
            && let Err(error) = is_dns1123_label(text)
        {
            let message = format!("`{}` is not a valid namespace name: {error}", shown(text));
            report(cx, namespace, text, message, NameFormat::Label);
        }
        let Some(kind) = object.builtin_kind() else {
            return;
        };
        let Some(name) = object.name() else {
            return;
        };
        let Some(text) = string_value(name) else {
            return;
        };
        let format = name_format(kind);
        let Err(reason) = check(text, format) else {
            return;
        };
        let message = format!(
            "`{}` is not a valid `{}` name: {reason}",
            shown(text),
            kind.as_str()
        );
        report(cx, name, text, message, format);
    }
}

fn report(
    cx: &mut ObjectContext<'_>,
    node: &Node,
    text: &str,
    message: String,
    format: NameFormat,
) {
    let max = format.max_len();
    let help = match (format, suggestion(text, format)) {
        _ if text.len() > max => format!("shorten the name to at most {max} characters"),
        (NameFormat::PathSegment, _) => {
            "remove `/` and `%` from the name; other characters are allowed".to_owned()
        }
        (_, Some(name)) => format!("use lowercase letters, digits and `-`, for example `{name}`"),
        (NameFormat::Dns1035, None) => {
            "use lowercase letters, digits and `-`, starting with a letter".to_owned()
        }
        (_, None) => "use lowercase letters, digits and `-`, starting and ending with a \
                      letter or digit"
            .to_owned(),
    };
    cx.report(cx.diagnostic(node.range, message).with_help(help));
}

fn name_format(kind: BuiltinKind) -> NameFormat {
    match kind {
        BuiltinKind::Namespace => NameFormat::Label,
        BuiltinKind::Service => NameFormat::Dns1035,
        BuiltinKind::Role
        | BuiltinKind::ClusterRole
        | BuiltinKind::RoleBinding
        | BuiltinKind::ClusterRoleBinding => NameFormat::PathSegment,
        _ => NameFormat::Subdomain,
    }
}

/// The text of a non-empty string value. Numbers, booleans and collections are schema
/// problems; empty names are KL1003's.
fn string_value(node: &Node) -> Option<&str> {
    let NodeKind::Scalar(scalar) = &node.kind else {
        return None;
    };
    (scalar.resolve_kubernetes() == ResolvedScalar::String && !scalar.value.trim().is_empty())
        .then_some(scalar.value.as_str())
}

fn check(name: &str, format: NameFormat) -> Result<(), String> {
    let result: Result<(), ValidationError> = match format {
        NameFormat::Subdomain => is_dns1123_subdomain(name),
        NameFormat::Label => is_dns1123_label(name),
        NameFormat::Dns1035 => is_dns1035_label(name),
        NameFormat::PathSegment => return check_path_segment(name),
    };
    result.map_err(|error| error.to_string())
}

/// `IsValidPathSegmentName` in Kubernetes' `apimachinery/pkg/api/validation/path`.
fn check_path_segment(name: &str) -> Result<(), String> {
    if name == "." || name == ".." {
        return Err(format!("may not be '{name}'"));
    }
    for illegal in ['/', '%'] {
        if name.contains(illegal) {
            return Err(format!("may not contain '{illegal}'"));
        }
    }
    Ok(())
}

/// A valid name close to `name`: lowercased, with runs of other characters turned into
/// one `-` (or kept as `.` for subdomains), trimmed and shortened. `None` when nothing
/// usable is left, and for non-ASCII names, where dropping letters (`café` to `caf`)
/// would change the word.
fn suggestion(name: &str, format: NameFormat) -> Option<String> {
    if matches!(format, NameFormat::PathSegment) || !name.is_ascii() {
        return None;
    }
    let keep_dots = matches!(format, NameFormat::Subdomain);
    let max = format.max_len();
    let mut out = String::new();
    for c in name.chars() {
        let c = c.to_ascii_lowercase();
        let mapped = if c.is_ascii_lowercase() || c.is_ascii_digit() || (keep_dots && c == '.') {
            c
        } else {
            '-'
        };
        // Collapse a run of separators into one, a `.` if the run has one.
        let separator = |c: char| c == '-' || c == '.';
        if separator(mapped) && out.ends_with(separator) {
            if mapped == '.' {
                out.pop();
                out.push('.');
            }
            continue;
        }
        out.push(mapped);
    }
    let trimmed = out.trim_matches(['-', '.']);
    let shortened = trimmed
        .get(..max.min(trimmed.len()))?
        .trim_end_matches(['-', '.']);
    check(shortened, format)
        .is_ok()
        .then(|| shortened.to_owned())
}

#[cfg(test)]
mod tests;
