//! Span-preserving YAML parsing.
//!
//! Turns YAML text into a tree of nodes where every key, value and collection carries the
//! byte range it came from. Built on `saphyr-parser` events (ADR 0002).
//!
//! Allowed internal dependencies: `kubelint_source`. See `docs/architecture.md`.
