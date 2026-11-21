//! KL1008 `invalid-object-name`.

use std::borrow::Cow;

use kubelint_diagnostics::Severity;
use kubelint_k8s::{
    BuiltinKind, MAX_LABEL_LENGTH, MAX_SUBDOMAIN_LENGTH, Object, ValidationError, is_dns1035_label,
    is_dns1123_label, is_dns1123_subdomain,
};
use kubelint_schema::KubernetesVersion;
use kubelint_yaml::{Node, NodeKind, ResolvedScalar};

use super::{has_placeholder, shown};
use crate::{Category, ObjectContext, Rule, RuleMeta};

/// Reports a `metadata.name` or `metadata.generateName` that the API server would reject
/// for the object's kind, and a `metadata.namespace` that is not a valid namespace name.
///
/// Names are checked only for built-in kinds ([`BuiltinKind`], in their own API group),
/// because a custom resource's name rules are up to its definition. Most kinds need a
/// DNS-1123 subdomain, `Namespace` and `StatefulSet` a DNS-1123 label, `Service` a
/// DNS-1035 label before Kubernetes 1.36 and a DNS-1123 label from it, and the RBAC
/// kinds and `PodDisruptionBudget` only a valid path segment (so `system:controller:foo`
/// is fine). A `CronJob` name has at most 52 characters and a `Job` name at most 63,
/// unless `spec.manualSelector` is `true`. A `generateName` is a prefix and may end with
/// `-`. A namespace is not checked on a listed cluster-scoped kind, where the server
/// ignores it. Values that are not strings are schema problems and are skipped, as are
/// empty names (KL1003).
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

/// Characters the server appends to a `generateName`.
const RANDOM_LENGTH: usize = 5;
/// The server cuts a longer prefix to this length before it appends the random characters.
const GENERATED_STEM_LENGTH: usize = MAX_LABEL_LENGTH - RANDOM_LENGTH;

/// What a name, a `generateName` or a namespace must follow.
#[derive(Debug, Clone, Copy)]
struct NameRule {
    format: NameFormat,
    /// A `generateName`: a trailing `-` is allowed.
    prefix: bool,
    /// The server also checks the name it generates from the prefix, which it does only
    /// when the object has no `name`.
    generated: bool,
    /// A shorter limit than the format's, counted on the text as written.
    max_len: Option<usize>,
}

impl NameRule {
    fn of(format: NameFormat) -> Self {
        Self {
            format,
            prefix: false,
            generated: false,
            max_len: None,
        }
    }

    /// The rule for the `metadata.name` of `kind`.
    fn name(kind: BuiltinKind, object: Object<'_>, version: KubernetesVersion) -> Self {
        let format = match kind {
            BuiltinKind::Namespace | BuiltinKind::StatefulSet => NameFormat::Label,
            // `ValidateServiceCreate` checks a DNS-1123 label once the
            // `RelaxedServiceNameValidation` gate is on by default, from 1.36.
            BuiltinKind::Service if version.minor() >= 36 => NameFormat::Label,
            BuiltinKind::Service => NameFormat::Dns1035,
            BuiltinKind::Role
            | BuiltinKind::ClusterRole
            | BuiltinKind::RoleBinding
            | BuiltinKind::ClusterRoleBinding
            | BuiltinKind::PodDisruptionBudget => NameFormat::PathSegment,
            _ => NameFormat::Subdomain,
        };
        let max_len = match kind {
            // `ValidateCronJobCreate` in `pkg/apis/batch/validation`: the controller adds an
            // 11 character suffix to the name of each Job it creates.
            BuiltinKind::CronJob => Some(52),
            // The name is the `job-name` label value of the pod template, which the server
            // adds unless `spec.manualSelector` is true (`generateSelector` in
            // `pkg/registry/batch/job/strategy.go`).
            BuiltinKind::Job if generates_selector(object) => Some(MAX_LABEL_LENGTH),
            _ => None,
        };
        Self {
            max_len,
            ..Self::of(format)
        }
    }

    /// The rule for the `metadata.generateName` of `kind`, or `None` when the server
    /// does not check it.
    fn generate_name(
        kind: BuiltinKind,
        object: Object<'_>,
        version: KubernetesVersion,
    ) -> Option<Self> {
        let has_name = object
            .name()
            .and_then(Node::as_str)
            .is_some_and(|name| !name.trim().is_empty());
        // `ValidateReplicationController` skips `generateName` from 1.35 on.
        if has_name && kind == BuiltinKind::ReplicationController && version.minor() >= 35 {
            return None;
        }
        let name = Self::name(kind, object, version);
        // The generated name has at most 63 characters, so only a lower limit binds.
        let max_len = name
            .max_len
            .filter(|max| *max < MAX_LABEL_LENGTH)
            .map(|max| max - RANDOM_LENGTH);
        Some(Self {
            prefix: true,
            generated: !has_name,
            max_len: max_len.filter(|_| !has_name),
            ..Self::of(name.format)
        })
    }

    /// The longest name or prefix allowed.
    fn limit(self) -> usize {
        self.max_len
            .map_or(self.format.max_len(), |max| max.min(self.format.max_len()))
    }

    /// Whether `text` is over the limit, so that "shorten it" is the right advice.
    fn too_long(self, text: &str) -> bool {
        let len = if self.prefix {
            mask_trailing_dash(text).len()
        } else {
            text.len()
        };
        len > self.format.max_len() || self.max_len.is_some_and(|max| text.len() > max)
    }

    fn check(self, text: &str) -> Result<(), String> {
        let masked = if self.prefix {
            mask_trailing_dash(text)
        } else {
            Cow::Borrowed(text)
        };
        if let Err(reason) = check(&masked, self.format) {
            return Err(match self.format {
                NameFormat::PathSegment => reason,
                _ if self.prefix && !self.too_long(text) => prefix_reason(self.format),
                _ => reason,
            });
        }
        if self.generated {
            let stem = text.get(..GENERATED_STEM_LENGTH).unwrap_or(text);
            let generated = format!("{stem}{}", "a".repeat(RANDOM_LENGTH));
            if check(&generated, self.format).is_err() {
                return Err(match self.format {
                    NameFormat::Subdomain => "a part between dots cannot start with `-`".to_owned(),
                    format => prefix_reason(format),
                });
            }
        }
        match self.max_len {
            Some(max) if text.len() > max => Err(format!("must be no more than {max} characters")),
            _ => Ok(()),
        }
    }
}

impl Rule for InvalidObjectName {
    fn meta(&self) -> &'static RuleMeta {
        &META
    }

    fn check_object(&self, cx: &mut ObjectContext<'_>) {
        let object = cx.object();
        if !object.is_cluster_scoped()
            && let Some(namespace) = object.namespace()
        {
            let what = "in `metadata.namespace` is not a valid namespace name".to_owned();
            check_node(cx, namespace, NameRule::of(NameFormat::Label), &what);
        }
        let Some(kind) = object.builtin_kind() else {
            return;
        };
        let version = cx.kubernetes_version();
        if let Some(name) = object.name() {
            let what = format!("is not a valid `{}` name", kind.as_str());
            check_node(cx, name, NameRule::name(kind, object, version), &what);
        }
        if let Some(generate_name) = object.generate_name()
            && let Some(rule) = NameRule::generate_name(kind, object, version)
        {
            let what = format!(
                "in `metadata.generateName` is not a valid `{}` name prefix",
                kind.as_str()
            );
            check_node(cx, generate_name, rule, &what);
        }
    }
}

/// Reports `node` as `what` when its text breaks `rule`.
fn check_node(cx: &mut ObjectContext<'_>, node: &Node, rule: NameRule, what: &str) {
    let Some(text) = string_value(node) else {
        return;
    };
    let Err(reason) = rule.check(text) else {
        return;
    };
    let message = format!("`{}` {what}: {reason}", shown(text));
    let format = rule.format;
    let noun = if rule.prefix { "prefix" } else { "name" };
    let stem = if rule.prefix {
        text.trim_end_matches('-')
    } else {
        text
    };
    let suggestion = suggestion(stem, format)
        .map(|name| {
            if rule.prefix {
                format!("{name}-")
            } else {
                name
            }
        })
        .filter(|name| rule.check(name).is_ok());
    let help = match (format, suggestion) {
        _ if rule.too_long(text) => {
            format!("shorten the {noun} to at most {} characters", rule.limit())
        }
        (NameFormat::PathSegment, _) => {
            format!("remove `/` and `%` from the {noun}; other characters are allowed")
        }
        (NameFormat::Dns1035, None) => {
            format!("use {}, starting with a letter", allowed(format))
        }
        (_, Some(name)) => format!("use {}, for example `{name}`", allowed(format)),
        (_, None) if rule.prefix => format!(
            "use {}, starting with a letter or digit, and optionally ending with `-`",
            allowed(format)
        ),
        (_, None) => format!(
            "use {}, starting and ending with a letter or digit",
            allowed(format)
        ),
    };
    cx.report(cx.diagnostic(node.range, message).with_help(help));
}

/// The characters a name may contain.
fn allowed(format: NameFormat) -> &'static str {
    match format {
        NameFormat::Subdomain => "lowercase letters, digits, `-` and `.`",
        NameFormat::Label | NameFormat::Dns1035 => "lowercase letters, digits and `-`",
        NameFormat::PathSegment => "any characters except `/` and `%`",
    }
}

/// What is wrong with a prefix that has bad characters. It may end with `-`, so the plain
/// format description would say too much.
fn prefix_reason(format: NameFormat) -> String {
    let start = match format {
        NameFormat::Subdomain => "each part between dots must start with a letter or digit",
        NameFormat::Dns1035 => "start with a letter",
        _ => "start with a letter or digit",
    };
    format!("must contain only {}, and {start}", allowed(format))
}

/// Whether the Job's selector and pod template labels are generated: `spec.manualSelector`
/// is absent or `false`. Any other value is not decided.
fn generates_selector(object: Object<'_>) -> bool {
    object
        .node()
        .get_path(&["spec", "manualSelector"])
        .is_none_or(|manual| manual.as_bool() == Some(false))
}

/// `maskTrailingDash` in `apimachinery/pkg/api/validation/generic.go`: a prefix is checked
/// with its last character before a trailing `-` replaced by `a`, so the dash stands for
/// the random suffix and does not count towards the length.
fn mask_trailing_dash(text: &str) -> Cow<'_, str> {
    match text.strip_suffix('-') {
        Some(stem) if text.is_ascii() && !stem.is_empty() => {
            Cow::Owned(format!("{}a", &stem[..stem.len() - 1]))
        }
        _ => Cow::Borrowed(text),
    }
}

/// The text of a non-empty string value without a template placeholder. Numbers,
/// booleans and collections are schema problems; empty names are KL1003's.
fn string_value(node: &Node) -> Option<&str> {
    let NodeKind::Scalar(scalar) = &node.kind else {
        return None;
    };
    let text = scalar.value.as_str();
    (scalar.resolve_kubernetes() == ResolvedScalar::String
        && !text.trim().is_empty()
        && !has_placeholder(text))
    .then_some(text)
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

/// [`suggestion`] for a DNS subdomain, such as a label key prefix.
pub(super) fn subdomain_suggestion(name: &str) -> Option<String> {
    suggestion(name, NameFormat::Subdomain)
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
