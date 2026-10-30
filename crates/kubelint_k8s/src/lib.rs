//! Kubernetes object model.
//!
//! Read-only views over YAML documents that understand Kubernetes: `apiVersion`/`kind`,
//! metadata, and helpers that find pod specs and containers inside every workload kind.
//! Rules use these views instead of walking raw YAML.
//!
//! Allowed internal dependencies: `kubelint_source`, `kubelint_yaml`.
//! See `docs/architecture.md`.
