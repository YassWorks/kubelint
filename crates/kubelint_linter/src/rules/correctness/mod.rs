//! `KL1xxx`: objects the Kubernetes API server would reject, or that cannot work as
//! written.

mod duplicate_container_name;
mod duplicate_env_name;
mod duplicate_object;
mod invalid_label;
mod invalid_object_name;
mod invalid_resource_quantity;
mod missing_api_version;
mod missing_kind;
mod missing_object_name;
mod non_string_env_value;
mod selector_mismatch;
#[cfg(test)]
mod tests;

pub use duplicate_container_name::DuplicateContainerName;
pub use duplicate_env_name::DuplicateEnvName;
pub use duplicate_object::DuplicateObject;
pub use invalid_label::InvalidLabel;
pub use invalid_object_name::InvalidObjectName;
pub use invalid_resource_quantity::InvalidResourceQuantity;
pub use missing_api_version::MissingApiVersion;
pub use missing_kind::MissingKind;
pub use missing_object_name::MissingObjectName;
pub use non_string_env_value::NonStringEnvValue;
pub use selector_mismatch::SelectorMismatch;

use kubelint_k8s::Object;
use kubelint_source::TextRange;
use kubelint_yaml::{Entry, Node, NodeKind, ResolvedScalar};

/// What is wrong with one of the two type fields, `apiVersion` and `kind`.
enum TypeField<'a> {
    /// The key is absent.
    Missing,
    /// The key is there, but its value is null, empty or only whitespace. Kubernetes
    /// treats this exactly like a missing field.
    Empty(&'a Entry),
}

/// Checks the type field `key` of `object`. `None` when it has a usable value.
///
/// A value of the wrong type (a number, a mapping) is not reported here: that is a
/// schema problem, and the rule that cannot decide stays silent.
fn check_type_field<'a>(object: Object<'a>, key: &str) -> Option<TypeField<'a>> {
    let Some(entry) = object.node().get_entry(key) else {
        return Some(TypeField::Missing);
    };
    is_empty(&entry.value).then_some(TypeField::Empty(entry))
}

fn is_empty(value: &Node) -> bool {
    match &value.kind {
        NodeKind::Null => true,
        NodeKind::Scalar(scalar) => {
            scalar.resolve_kubernetes() == ResolvedScalar::Null || scalar.value.trim().is_empty()
        }
        NodeKind::Mapping(_) | NodeKind::Sequence(_) => false,
    }
}

/// The range a diagnostic about `entry` points at: the whole `key: value` pair, or just
/// the key when the value is missing (its range is empty and sits after the colon).
fn entry_range(entry: &Entry) -> TextRange {
    if entry.value.is_null() {
        entry.key.range
    } else {
        entry.key.range.cover(entry.value.range)
    }
}

/// Where a diagnostic about a missing field of `object` points: the `other` type field
/// when present (`kind: Deployment` for a missing `apiVersion`), otherwise the object's
/// first key.
fn missing_field_range(object: Object<'_>, other: &str) -> TextRange {
    if let Some(entry) = object.node().get_entry(other) {
        return entry_range(entry);
    }
    object
        .node()
        .entries()
        .next()
        .map_or(object.range(), |entry| entry.key.range)
}

/// `text` for a message: values longer than 40 characters are cut after 32, so a
/// 250-character name does not swamp the line.
fn shown(text: &str) -> String {
    match text.char_indices().nth(32) {
        Some((end, _)) if text.chars().count() > 40 => format!("{}...", &text[..end]),
        _ => text.to_owned(),
    }
}

/// Whether `value` holds a placeholder of a template tool (`{{ .Values.name }}`,
/// `${NAME}`, `$(NAME)`, `$NAME`), so its final text is unknown.
fn has_placeholder(value: &str) -> bool {
    value.contains("{{")
        || value.split('$').skip(1).any(|rest| {
            rest.starts_with(|c: char| matches!(c, '{' | '(' | '_') || c.is_ascii_alphabetic())
        })
}
