use std::path::{Path, PathBuf};

use crate::LineIndex;

/// Identifies one file within a lint run.
///
/// Diagnostics carry a `FileId` instead of a path so they stay small and cheap to copy.
/// The id is only meaningful together with the table that issued it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct FileId(pub u32);

/// A file's path and its full text.
///
/// The text is kept exactly as read, so every `TextRange` is a valid byte range into
/// [`SourceFile::text`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceFile {
    path: PathBuf,
    text: String,
    line_index: LineIndex,
}

impl SourceFile {
    /// Creates a source file and indexes its lines.
    ///
    /// ```
    /// use kubelint_source::SourceFile;
    ///
    /// let file = SourceFile::new("deploy.yaml", "kind: Pod\n");
    /// assert_eq!(file.text(), "kind: Pod\n");
    /// assert_eq!(file.line_index().line_count(), 2);
    /// ```
    pub fn new(path: impl Into<PathBuf>, text: impl Into<String>) -> Self {
        let text = text.into();
        let line_index = LineIndex::new(&text);
        Self {
            path: path.into(),
            text,
            line_index,
        }
    }

    /// The path as given by the user (or `<stdin>`), used for display.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// The full file text.
    pub fn text(&self) -> &str {
        &self.text
    }

    /// The line index for [`SourceFile::text`].
    pub fn line_index(&self) -> &LineIndex {
        &self.line_index
    }
}
