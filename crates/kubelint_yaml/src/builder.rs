//! Builds the span-preserving tree from `saphyr-parser` events.

use std::collections::HashMap;

use kubelint_source::{TextRange, TextSize};
use saphyr_parser::{Event, Parser, ScalarStyle as SaphyrStyle, Span};

use crate::{Document, Entry, Node, NodeKind, Scalar, ScalarStyle};

/// A YAML syntax error with the place it was found.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseError {
    /// What went wrong, as reported by the parser.
    pub message: String,
    /// Where it went wrong. Empty: it marks a position, not a span.
    pub range: TextRange,
}

/// The result of [`parse`]: every document that was read before the first error, plus
/// that error if there was one.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Parse {
    /// Completed documents in source order.
    pub documents: Vec<Document>,
    /// Syntax errors. The parser cannot resume after one, so this has at most one entry.
    pub errors: Vec<ParseError>,
}

/// Parses a YAML stream into documents.
///
/// A syntax error ends parsing: documents before it are kept (so a file with one broken
/// document still gets linted up to that point), the document containing the error is
/// dropped.
///
/// ```
/// let parse = kubelint_yaml::parse("a: 1\n---\nb: 2\n");
/// assert_eq!(parse.documents.len(), 2);
/// assert!(parse.errors.is_empty());
/// ```
pub fn parse(text: &str) -> Parse {
    let offsets = ByteOffsets::new(text);
    let mut builder = Builder::new(text, &offsets);
    let mut out = Parse::default();
    for item in Parser::new_from_str(text) {
        match item {
            Ok((event, span)) => {
                if let Some(doc) = builder.event(event, span) {
                    out.documents.push(doc);
                }
            }
            Err(err) => {
                let at = offsets.get(err.marker().index());
                out.errors.push(ParseError {
                    message: err.info().to_owned(),
                    range: TextRange::empty(at),
                });
                break;
            }
        }
    }
    out
}

/// Maps character indices (what `saphyr-parser` reports) to byte offsets.
struct ByteOffsets {
    /// `None` when the text is ASCII and indices already are byte offsets.
    table: Option<Vec<u32>>,
    len: u32,
}

impl ByteOffsets {
    fn new(text: &str) -> Self {
        let len = u32::try_from(text.len()).unwrap_or(u32::MAX);
        let table = (!text.is_ascii()).then(|| {
            text.char_indices()
                .map(|(i, _)| u32::try_from(i).unwrap_or(u32::MAX))
                .chain(std::iter::once(len))
                .collect()
        });
        Self { table, len }
    }

    /// Byte offset of a character index. Out-of-range indices clamp to the end.
    fn get(&self, char_index: usize) -> TextSize {
        let byte = match &self.table {
            None => u32::try_from(char_index).unwrap_or(u32::MAX).min(self.len),
            Some(t) => t.get(char_index).copied().unwrap_or(self.len),
        };
        TextSize::new(byte)
    }

    fn range(&self, span: Span) -> TextRange {
        let start = self.get(span.start.index());
        let end = self.get(span.end.index()).max(start);
        TextRange::new(start, end)
    }
}

enum Frame {
    Seq {
        flow: bool,
        start: TextSize,
        anchor: usize,
        items: Vec<Node>,
    },
    Map {
        flow: bool,
        start: TextSize,
        anchor: usize,
        entries: Vec<Entry>,
        key: Option<Node>,
    },
}

struct Builder<'a> {
    text: &'a str,
    offsets: &'a ByteOffsets,
    stack: Vec<Frame>,
    root: Option<Node>,
    doc_range: TextRange,
    anchors: HashMap<usize, Node>,
}

impl<'a> Builder<'a> {
    fn new(text: &'a str, offsets: &'a ByteOffsets) -> Self {
        Self {
            text,
            offsets,
            stack: Vec::new(),
            root: None,
            doc_range: TextRange::default(),
            anchors: HashMap::new(),
        }
    }

    /// Feeds one event; returns a document when it ends.
    fn event(&mut self, event: Event<'_>, span: Span) -> Option<Document> {
        let range = self.offsets.range(span);
        match event {
            Event::DocumentStart(_) => {
                self.stack.clear();
                self.anchors.clear();
                self.root = None;
                self.doc_range = range;
            }
            Event::DocumentEnd => {
                let root = self.root.take();
                let end = root
                    .as_ref()
                    .map_or(self.doc_range.end(), |r| r.range.end());
                let range = TextRange::new(self.doc_range.start(), end.max(self.doc_range.start()));
                return Some(Document { root, range });
            }
            Event::Scalar(value, style, anchor, tag) => {
                let start = if anchor != 0 || tag.is_some() {
                    self.skip_properties(range.start())
                } else {
                    range.start()
                };
                let range = TextRange::new(start, range.end().max(start));
                let node = if style == SaphyrStyle::Plain && value.is_empty() {
                    Node {
                        kind: NodeKind::Null,
                        range: TextRange::empty(start),
                    }
                } else {
                    Node {
                        kind: NodeKind::Scalar(Scalar {
                            value: value.into_owned(),
                            style: convert_style(style),
                        }),
                        range,
                    }
                };
                self.finish(node, anchor);
            }
            Event::Alias(id) => {
                let node = self.anchors.get(&id).cloned().unwrap_or(Node {
                    kind: NodeKind::Null,
                    range,
                });
                self.finish(node, 0);
            }
            Event::SequenceStart(anchor, tag) => {
                let flow = self.is_flow(range.start(), anchor, tag.is_some());
                self.stack.push(Frame::Seq {
                    flow: flow.is_some(),
                    start: flow.unwrap_or(range.start()),
                    anchor,
                    items: Vec::new(),
                });
            }
            Event::MappingStart(anchor, tag) => {
                let flow = self.is_flow(range.start(), anchor, tag.is_some());
                self.stack.push(Frame::Map {
                    flow: flow.is_some(),
                    start: flow.unwrap_or(range.start()),
                    anchor,
                    entries: Vec::new(),
                    key: None,
                });
            }
            Event::SequenceEnd | Event::MappingEnd => {
                if let Some(frame) = self.stack.pop() {
                    let (node, anchor) = close(frame, range.end());
                    self.finish(node, anchor);
                }
            }
            Event::StreamStart | Event::StreamEnd | Event::Nothing => {}
        }
        None
    }

    /// For a collection start: the start of its `[`/`{` when it is a flow collection.
    fn is_flow(&self, at: TextSize, anchor: usize, tagged: bool) -> Option<TextSize> {
        let at = if anchor != 0 || tagged {
            self.skip_properties(at)
        } else {
            at
        };
        let byte = self.text.as_bytes().get(usize::from(at)).copied();
        matches!(byte, Some(b'[' | b'{')).then_some(at)
    }

    /// Skips `&anchor` and `!tag` prefixes (the parser includes them in a node's span).
    fn skip_properties(&self, from: TextSize) -> TextSize {
        let bytes = self.text.as_bytes();
        let mut pos = usize::from(from);
        while matches!(bytes.get(pos), Some(b'&' | b'!')) {
            while bytes.get(pos).is_some_and(|b| !b.is_ascii_whitespace()) {
                pos += 1;
            }
            while bytes.get(pos).is_some_and(u8::is_ascii_whitespace) {
                pos += 1;
            }
        }
        TextSize::new(u32::try_from(pos).unwrap_or(u32::MAX))
    }

    /// Attaches a finished node to its parent (or makes it the document root).
    fn finish(&mut self, node: Node, anchor: usize) {
        if anchor != 0 {
            self.anchors.insert(anchor, node.clone());
        }
        match self.stack.last_mut() {
            None => {
                // An empty document is reported as an empty plain scalar.
                let empty = matches!(node.kind, NodeKind::Null);
                self.root = (!empty).then_some(node);
            }
            Some(Frame::Seq { items, .. }) => items.push(node),
            Some(Frame::Map { entries, key, .. }) => match key.take() {
                None => *key = Some(node),
                Some(key) => entries.push(Entry { key, value: node }),
            },
        }
    }
}

/// Closes a collection. Flow collections span `[`..`]`; block collections span their
/// children, because the parser reports empty spans for block starts and ends.
fn close(frame: Frame, end_marker: TextSize) -> (Node, usize) {
    let (flow, start, anchor, kind, child_range) = match frame {
        Frame::Seq {
            flow,
            start,
            anchor,
            items,
        } => {
            let r = cover(items.iter().map(|n| n.range));
            (flow, start, anchor, NodeKind::Sequence(items), r)
        }
        Frame::Map {
            flow,
            start,
            anchor,
            entries,
            ..
        } => {
            let r = cover(entries.iter().flat_map(|e| [e.key.range, e.value.range]));
            (flow, start, anchor, NodeKind::Mapping(entries), r)
        }
    };
    let range = if flow {
        TextRange::new(start, end_marker.max(start))
    } else {
        child_range.unwrap_or_else(|| TextRange::empty(start))
    };
    (Node { kind, range }, anchor)
}

fn cover(ranges: impl Iterator<Item = TextRange>) -> Option<TextRange> {
    ranges.reduce(TextRange::cover)
}

fn convert_style(style: SaphyrStyle) -> ScalarStyle {
    match style {
        SaphyrStyle::Plain => ScalarStyle::Plain,
        SaphyrStyle::SingleQuoted => ScalarStyle::SingleQuoted,
        SaphyrStyle::DoubleQuoted => ScalarStyle::DoubleQuoted,
        SaphyrStyle::Literal => ScalarStyle::Literal,
        SaphyrStyle::Folded => ScalarStyle::Folded,
    }
}
