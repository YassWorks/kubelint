//! Kubernetes object model.
//!
//! Read-only views over YAML documents that understand Kubernetes: `apiVersion`/`kind`,
//! metadata, helpers that find pod specs and containers inside every workload kind, and
//! parsers and validators for Kubernetes value formats such as resource quantities,
//! DNS names and labels.
//! Rules use these views instead of walking raw YAML.
//!
//! Allowed internal dependencies: `kubelint_source`, `kubelint_yaml`.
//! See `docs/architecture.md`.

mod gvk;
mod object;
mod quantity;
mod validation;
mod workload;

pub use gvk::{GroupVersion, GroupVersionKind, stable_api_version};
pub use object::{Object, objects};
pub use quantity::{Quantity, QuantityError, Suffix};
pub use validation::{
    Format, KeyPart, MAX_LABEL_LENGTH, MAX_SUBDOMAIN_LENGTH, ValidationError, ValidationErrorKind,
    is_dns1035_label, is_dns1123_label, is_dns1123_subdomain, is_qualified_name,
    is_valid_label_value,
};
pub use workload::{Container, ContainerKind, PodSpec};
