use std::fmt::Write as _;

use kubelint_source::TextRange;

/// One YAML document of a stream (`---` separates documents).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Document {
    /// The top-level node, or `None` for an empty document.
    pub root: Option<Node>,
    /// The byte range of the whole document.
    pub range: TextRange,
}

/// A YAML node together with the byte range it occupies in the source text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Node {
    /// What the node contains.
    pub kind: NodeKind,
    /// The byte range of the node, including quotes for quoted scalars.
    pub range: TextRange,
}

/// The shape of a [`Node`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NodeKind {
    /// Entries in source order. Duplicate keys are kept, because KL0002 reports them.
    Mapping(Vec<Entry>),
    /// Items in source order.
    Sequence(Vec<Node>),
    /// A string-like leaf. Its type (number, bool...) is decided later, see [`Scalar`].
    Scalar(Scalar),
    /// An explicit `~` or `null`, or a missing value as in `key:`.
    Null,
}

/// One `key: value` pair of a mapping. Keys are nodes too, so a diagnostic can point at
/// either side.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    /// The key node.
    pub key: Node,
    /// The value node.
    pub value: Node,
}

/// The text of a scalar and how it was written.
///
/// The raw text is kept untyped on purpose: Kubernetes decides whether `yes` or `8080` is
/// a string, a bool or a number, and rules such as KL1009 need to see the original form.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Scalar {
    /// The scalar's value with quotes and escapes already processed.
    pub value: String,
    /// How the scalar was written in the source.
    pub style: ScalarStyle,
}

/// The YAML scalar styles. Quoted scalars are always strings in Kubernetes, plain ones
/// are guessed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScalarStyle {
    /// Unquoted: `key: value`.
    Plain,
    /// `'value'`.
    SingleQuoted,
    /// `"value"`.
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
        let mut out = String::new();
        let _ = writeln!(out, "document {}", fmt_range(self.range));
        match &self.root {
            Some(root) => root.dump_into(&mut out, 1),
            None => out.push_str("  (empty)\n"),
        }
        out
    }
}

impl Node {
    fn dump_into(&self, out: &mut String, depth: usize) {
        let pad = "  ".repeat(depth);
        let range = fmt_range(self.range);
        match &self.kind {
            NodeKind::Null => {
                let _ = writeln!(out, "{pad}null {range}");
            }
            NodeKind::Scalar(s) => {
                let _ = writeln!(out, "{pad}scalar {:?} {:?} {range}", s.style, s.value);
            }
            NodeKind::Sequence(items) => {
                let _ = writeln!(out, "{pad}sequence {range}");
                for item in items {
                    item.dump_into(out, depth + 1);
                }
            }
            NodeKind::Mapping(entries) => {
                let _ = writeln!(out, "{pad}mapping {range}");
                for entry in entries {
                    let _ = writeln!(out, "{pad}  entry");
                    entry.key.dump_into(out, depth + 2);
                    entry.value.dump_into(out, depth + 2);
                }
            }
        }
    }
}

fn fmt_range(range: TextRange) -> String {
    format!("{}..{}", u32::from(range.start()), u32::from(range.end()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn range(start: u32, end: u32) -> TextRange {
        TextRange::new(start.into(), end.into())
    }

    fn scalar(value: &str, style: ScalarStyle, r: TextRange) -> Node {
        Node {
            kind: NodeKind::Scalar(Scalar {
                value: value.to_owned(),
                style,
            }),
            range: r,
        }
    }

    #[test]
    fn empty_document() {
        let doc = Document {
            root: None,
            range: range(0, 0),
        };
        assert_eq!(doc.dump(), "document 0..0\n  (empty)\n");
    }

    #[test]
    fn dump_of_nested_tree_with_non_ascii() {
        // Source: "name: é\nports: [80]\nnone:\n" with byte offsets (é is 2 bytes).
        let doc = Document {
            root: Some(Node {
                kind: NodeKind::Mapping(vec![
                    Entry {
                        key: scalar("name", ScalarStyle::Plain, range(0, 4)),
                        value: scalar("é", ScalarStyle::DoubleQuoted, range(6, 9)),
                    },
                    Entry {
                        key: scalar("ports", ScalarStyle::Plain, range(10, 15)),
                        value: Node {
                            kind: NodeKind::Sequence(vec![scalar(
                                "80",
                                ScalarStyle::Plain,
                                range(18, 20),
                            )]),
                            range: range(17, 21),
                        },
                    },
                    Entry {
                        key: scalar("none", ScalarStyle::Plain, range(22, 26)),
                        value: Node {
                            kind: NodeKind::Null,
                            range: range(27, 27),
                        },
                    },
                ]),
                range: range(0, 27),
            }),
            range: range(0, 28),
        };
        let expected = "\
document 0..28
  mapping 0..27
    entry
      scalar Plain \"name\" 0..4
      scalar DoubleQuoted \"é\" 6..9
    entry
      scalar Plain \"ports\" 10..15
      sequence 17..21
        scalar Plain \"80\" 18..20
    entry
      scalar Plain \"none\" 22..26
      null 27..27
";
        assert_eq!(doc.dump(), expected);
    }
}
