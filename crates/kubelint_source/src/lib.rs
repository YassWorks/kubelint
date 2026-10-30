//! Source files and positions.
//!
//! Owns the source file type, byte-offset text ranges (re-exported from `text-size`)
//! and a line index that maps byte offsets to line/column pairs and back.
//!
//! Every other crate refers to locations through the types defined here. This crate has
//! no internal dependencies. See `docs/architecture.md`.

mod line_index;
mod source_file;

pub use line_index::{LineCol, LineIndex};
pub use source_file::{FileId, SourceFile};
pub use text_size::{TextRange, TextSize};
