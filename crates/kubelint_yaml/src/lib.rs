//! Span-preserving YAML parsing.
//!
//! Turns YAML text into a tree of nodes where every key, value and collection carries the
//! byte range it came from. Built on `saphyr-parser` events (ADR 0002).
//!
//! Allowed internal dependencies: `kubelint_source`. See `docs/architecture.md`.

mod builder;
mod nav;
mod resolve;
mod tree;

pub use builder::{Parse, ParseError, parse};
pub use resolve::ResolvedScalar;

pub use tree::{Document, Entry, Node, NodeKind, Scalar, ScalarStyle};
