//! Span-preserving YAML parsing.
//!
//! Turns YAML text into a tree of nodes where every key, value and collection carries the
//! byte range it came from. Built on `saphyr-parser` events.
//!
//! Allowed internal dependencies: `kubelint_source`. See `docs/architecture.md`.

mod builder;
mod comments;
mod nav;
mod resolve;
mod tree;

pub use builder::{Parse, ParseError, parse};
pub use comments::Comment;
pub use resolve::ResolvedScalar;

pub use tree::{Document, Entry, Node, NodeKind, Scalar, ScalarStyle};
