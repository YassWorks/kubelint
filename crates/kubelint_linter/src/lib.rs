//! The linter.
//!
//! Defines the `Rule` trait and rule metadata, holds the registry of built-in rules, applies
//! configuration and suppression comments, and runs rules over Kubernetes objects to produce
//! diagnostics.
//!
//! Allowed internal dependencies: `kubelint_source`, `kubelint_yaml`, `kubelint_k8s`,
//! `kubelint_diagnostics`, `kubelint_config`. See `docs/architecture.md`.
