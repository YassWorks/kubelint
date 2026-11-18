//! Kubernetes API schemas.
//!
//! The schema of one Kubernetes version, read from the compact JSON form that
//! `cargo xtask import-schemas` writes, and a lookup from an object's group, version and
//! kind to its definition.
//!
//! Allowed internal dependencies: `kubelint_source`, `kubelint_yaml`, `kubelint_k8s`.
//! See `docs/architecture.md`.

mod embedded;
mod model;

pub use embedded::{KubernetesVersion, VERSIONS};
pub use model::{
    Definition, DefinitionId, FORMAT_VERSION, Object, Scalar, Schema, SchemaError, Shape,
};
