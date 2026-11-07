//! KL1008 `invalid-object-name`.

use kubelint_diagnostics::Severity;
use kubelint_k8s::{
    GroupVersion, MAX_LABEL_LENGTH, MAX_SUBDOMAIN_LENGTH, Object, ValidationError,
    is_dns1035_label, is_dns1123_label, is_dns1123_subdomain, stable_api_version,
};
use kubelint_yaml::{Node, NodeKind, ResolvedScalar};

use super::shown;
use crate::{Category, ObjectContext, Rule, RuleMeta};

/// Reports a `metadata.name` that the API server would reject for the object's kind,
/// and a `metadata.namespace` that is not a valid namespace name.
///
/// Names are checked only for built-in kinds (those [`stable_api_version`] knows, in
/// their own API group), because a custom resource's name rules are up to its
/// definition. Most kinds need a DNS-1123 subdomain, `Namespace` a DNS-1123 label,
/// `Service` a DNS-1035 label, and the RBAC kinds only a valid path segment (so
/// `system:controller:foo` is fine). Values that are not strings are schema problems
/// and are skipped, as are empty names (KL1003).
pub struct InvalidObjectName;

static META: RuleMeta = RuleMeta {
    code: "KL1008",
    name: "invalid-object-name",
    category: Category::Correctness,
    default_severity: Severity::Error,
    default_enabled: true,
    summary: "An object name or namespace that Kubernetes would reject.",
    explanation: r"## What it does

Reports a `metadata.name` that is not valid for the object's kind, and a
`metadata.namespace` that is not a valid namespace name.

## Why it matters

Kubernetes uses names in DNS records, URLs and file paths, so it restricts them. Most
objects need a DNS-1123 subdomain: lowercase letters, digits, `-` and `.`, at most 253
characters, starting and ending with a letter or digit. Namespaces need a DNS-1123
label (no dots, at most 63 characters), and Services a DNS-1035 label, which must also
start with a letter. The API server rejects any other name, so names like `My_App` or
`web.` fail at apply time. Uppercase letters and underscores are the usual culprits.

## Example

```yaml
kind: Deployment
metadata:
  name: My_App
```

Use instead:

```yaml
kind: Deployment
metadata:
  name: my-app
```
",
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
            report(cx, namespace, message, NameFormat::Label);
        }
        let Some((kind, format)) = name_format(object) else {
            return;
        };
        let Some(name) = object.name() else {
            return;
        };
        let Some(text) = string_value(name) else {
            return;
        };
        let Err(reason) = check(text, format) else {
            return;
        };
        let message = format!("`{}` is not a valid `{kind}` name: {reason}", shown(text));
        report(cx, name, message, format);
    }
}

fn report(cx: &mut ObjectContext<'_>, node: &Node, message: String, format: NameFormat) {
    let text = node_text(node);
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
    let diagnostic = cx.diagnostic(node.range, message).with_help(help);
    cx.report(diagnostic);
}

/// The kind and its name format, for built-in kinds in their own API group. Without a
/// usable `apiVersion` the kind alone decides.
fn name_format(object: Object<'_>) -> Option<(&str, NameFormat)> {
    let kind = object.kind()?.as_str()?;
    let stable = GroupVersion::parse(stable_api_version(kind)?)?;
    if let Some(gv) = object.group_version()
        && gv.group != stable.group
    {
        return None;
    }
    let format = match kind {
        "Namespace" => NameFormat::Label,
        "Service" => NameFormat::Dns1035,
        "Role" | "ClusterRole" | "RoleBinding" | "ClusterRoleBinding" => NameFormat::PathSegment,
        _ => NameFormat::Subdomain,
    };
    Some((kind, format))
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

/// The scalar text of a node checked by [`string_value`].
fn node_text(node: &Node) -> &str {
    node.as_scalar().map_or("", |scalar| scalar.value.as_str())
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
mod tests {
    use super::{InvalidObjectName, NameFormat, suggestion};

    #[test]
    fn fixture() {
        insta::assert_snapshot!(crate::testing::check_fixture(&InvalidObjectName));
    }

    #[test]
    fn suggestions() {
        let cases = [
            ("My_App", NameFormat::Subdomain, Some("my-app")),
            ("web.", NameFormat::Subdomain, Some("web")),
            ("a__.b", NameFormat::Subdomain, Some("a.b")),
            ("api.v2", NameFormat::Label, Some("api-v2")),
            ("Web Server", NameFormat::Dns1035, Some("web-server")),
            ("1web", NameFormat::Dns1035, None),
            ("café", NameFormat::Dns1035, None),
            ("___", NameFormat::Subdomain, None),
            ("a/b", NameFormat::PathSegment, None),
        ];
        for (name, format, expected) in cases {
            assert_eq!(suggestion(name, format).as_deref(), expected, "{name}");
        }
        let long = "a".repeat(70);
        assert_eq!(suggestion(&long, NameFormat::Label), Some("a".repeat(63)));
    }
}
