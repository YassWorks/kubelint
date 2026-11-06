//! Kubernetes object model.
//!
//! Read-only views over YAML documents that understand Kubernetes: `apiVersion`/`kind`,
//! metadata, helpers that find pod specs and containers inside every workload kind, and
//! parsers for Kubernetes value formats such as resource quantities.
//! Rules use these views instead of walking raw YAML.
//!
//! Allowed internal dependencies: `kubelint_source`, `kubelint_yaml`.
//! See `docs/architecture.md`.

mod gvk;
mod object;
mod quantity;
mod workload;

pub use gvk::{GroupVersion, GroupVersionKind, stable_api_version};
pub use object::{Object, objects};
pub use quantity::{Quantity, QuantityError, Suffix};
pub use workload::{Container, ContainerKind, PodSpec};
