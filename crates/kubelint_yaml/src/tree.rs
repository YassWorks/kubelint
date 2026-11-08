use std::fmt;

use kubelint_source::{TextRange, TextSize};

/// One YAML document of a stream (`---` separates documents).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Document {
    /// `None` for an empty document.
    pub root: Option<Node>,
    pub range: TextRange,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Node {
    pub kind: NodeKind,
    /// Includes the quotes of a quoted scalar.
    pub range: TextRange,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NodeKind {
    /// Entries in source order. Duplicate keys are kept, because KL0002 reports them.
    Mapping(Vec<Entry>),
    Sequence(Vec<Node>),
    /// A string-like leaf. Its type (number, bool...) is decided later, see [`Scalar`].
    Scalar(Scalar),
    /// A missing value, as in `key:` or `- `. An explicit `~` or `null` is a plain
    /// [`Scalar`], because Kubernetes decides what it means.
    Null,
}

/// One `key: value` pair of a mapping. Keys are nodes too, so a diagnostic can point at
/// either side.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    pub key: Node,
    pub value: Node,
}

/// Untyped: Kubernetes decides whether `yes` or `8080` is a string, a bool or a number,
/// and rules such as KL1009 need to see the original form.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Scalar {
    /// Quotes and escapes already processed.
    pub value: String,
    pub style: ScalarStyle,
}

/// Quoted scalars are always strings in Kubernetes, plain ones are guessed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScalarStyle {
    Plain,
    SingleQuoted,
    DoubleQuoted,
    /// Block scalar introduced by `|`.
    Literal,
    /// Block scalar introduced by `>`.
    Folded,
}

impl Document {
    /// Renders the tree as indented text with ranges, for snapshot tests and debugging.
    ///
    /// ```
    /// use kubelint_source::TextRange;
    /// use kubelint_yaml::{Document, Node, NodeKind};
    ///
    /// let doc = Document {
    ///     root: Some(Node { kind: NodeKind::Null, range: TextRange::empty(0.into()) }),
    ///     range: TextRange::empty(0.into()),
    /// };
    /// assert_eq!(doc.dump(), "document 0..0\n  null 0..0\n");
    /// ```
    pub fn dump(&self) -> String {
        match &self.root {
            Some(root) => format!("document {:?}\n{}", self.range, Dump(root, 1)),
            None => format!("document {:?}\n  (empty)\n", self.range),
        }
    }

    /// Moves every range `delta` bytes later, for documents parsed from a suffix of the
    /// text.
    pub(crate) fn shifted(mut self, delta: TextSize) -> Self {
        self.range += delta;
        if let Some(root) = &mut self.root {
            root.shift(delta);
        }
        self
    }
}

impl Node {
    fn shift(&mut self, delta: TextSize) {
        self.range += delta;
        match &mut self.kind {
            NodeKind::Mapping(entries) => {
                for entry in entries {
                    entry.key.shift(delta);
                    entry.value.shift(delta);
                }
            }
            NodeKind::Sequence(items) => {
                for item in items {
                    item.shift(delta);
                }
            }
            NodeKind::Scalar(_) | NodeKind::Null => {}
        }
    }
}

/// A node rendered by [`Document::dump`], indented `depth` levels. Ranges print as
/// `start..end`.
struct Dump<'a>(&'a Node, usize);

impl fmt::Display for Dump<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Self(node, depth) = *self;
        let (pad, range) = ("  ".repeat(depth), node.range);
        match &node.kind {
            NodeKind::Null => writeln!(f, "{pad}null {range:?}"),
            NodeKind::Scalar(s) => writeln!(f, "{pad}scalar {:?} {:?} {range:?}", s.style, s.value),
            NodeKind::Sequence(items) => {
                writeln!(f, "{pad}sequence {range:?}")?;
                items
                    .iter()
                    .try_for_each(|item| write!(f, "{}", Dump(item, depth + 1)))
            }
            NodeKind::Mapping(entries) => {
                writeln!(f, "{pad}mapping {range:?}")?;
                entries.iter().try_for_each(|e| {
                    let (key, value) = (Dump(&e.key, depth + 2), Dump(&e.value, depth + 2));
                    write!(f, "{pad}  entry\n{key}{value}")
                })
            }
        }
    }
}
