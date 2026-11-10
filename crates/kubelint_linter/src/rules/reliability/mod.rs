//! `KL3xxx`: settings that are valid and safe, but likely to cause outages or
//! unpredictable behavior.

mod missing_memory_limit;
mod missing_readiness_probe;
mod missing_resource_requests;
mod unpinned_image_tag;

pub use missing_memory_limit::MissingMemoryLimit;
pub use missing_readiness_probe::MissingReadinessProbe;
pub use missing_resource_requests::MissingResourceRequests;
pub use unpinned_image_tag::UnpinnedImageTag;

use kubelint_source::TextRange;
use kubelint_yaml::Entry;

/// Whether `<section>.<resource>` (such as `limits.memory`) is set under a `resources`
/// entry.
enum ResourceField {
    /// The value is missing or null. The range is the deepest key on the way to it,
    /// the section or `resources`, when there is one.
    Missing(Option<TextRange>),
    /// The value is set, or a value on the way to it is a schema problem.
    Set,
}

impl ResourceField {
    fn lookup(resources: Option<&Entry>, section: &str, resource: &str) -> Self {
        let mut found = None;
        let mut entry = resources;
        for key in [section, resource] {
            let Some(current) = entry.filter(|entry| !entry.value.is_kubernetes_null()) else {
                return Self::Missing(found);
            };
            if current.value.as_mapping().is_none() {
                return Self::Set;
            }
            found = Some(current.key.range);
            entry = current.value.get_entry(key);
        }
        match entry {
            Some(value) if !value.value.is_kubernetes_null() => Self::Set,
            _ => Self::Missing(found),
        }
    }

    fn is_set(&self) -> bool {
        matches!(self, Self::Set)
    }
}
