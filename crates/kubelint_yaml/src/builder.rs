//! Builds the span-preserving tree from `saphyr-parser` events.

use std::collections::HashMap;

use kubelint_source::{TextRange, TextSize};
use saphyr_parser::{Event, Parser, ScalarStyle as SaphyrStyle, Span};

use crate::comments::{self, Comment};
use crate::{Document, Entry, Node, NodeKind, Scalar, ScalarStyle};

/// A YAML syntax error with the place it was found.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseError {
    /// What went wrong, as reported by the parser.
    pub message: String,
    /// Where it went wrong. Empty: it marks a position, not a span.
    pub range: TextRange,
}

/// The result of [`parse`]: every document that could be read, the syntax errors, and
/// the comments.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Parse {
    /// Completed documents in source order.
    pub documents: Vec<Document>,
    /// Syntax errors in source order, at most one per broken document.
    pub errors: Vec<ParseError>,
    /// Comments in source order. Comments between a syntax error and the next document
    /// marker are not reported, because the parser never told us which `#` there were
    /// inside scalars.
    pub comments: Vec<Comment>,
}

/// Parses a YAML stream into documents.
///
/// A syntax error drops the document that contains it. Parsing resumes at the next
/// document marker (`---` at the start of a line), so a file with one broken document
/// still gets its other documents linted. A marker at column 0 always starts a new
/// document in YAML, even inside a block or quoted scalar, which makes this safe.
///
/// ```
/// let parse = kubelint_yaml::parse("a: 1\n---\nb: 2\n");
/// assert_eq!(parse.documents.len(), 2);
/// assert!(parse.errors.is_empty());
///
/// let parse = kubelint_yaml::parse("a: [1\n---\nb: 2\n");
/// assert_eq!(parse.errors.len(), 1);
/// assert_eq!(parse.documents.len(), 1);
///
/// let parse = kubelint_yaml::parse("a: '#1' # note\n");
/// assert_eq!(parse.comments.len(), 1);
/// assert_eq!(parse.comments[0].text, " note");
/// ```
pub fn parse(text: &str) -> Parse {
    let mut out = Parse::default();
    let mut base = 0;
    while let Some(segment) = text.get(base..) {
        let part = parse_segment(segment);
        let delta = TextSize::new(u32::try_from(base).unwrap_or(u32::MAX));
        let error_at = part
            .errors
            .first()
            .map(|e| base + usize::from(e.range.start()));
        out.documents
            .extend(part.documents.into_iter().map(|d| d.shifted(delta)));
        out.errors
            .extend(part.errors.into_iter().map(|e| ParseError {
                message: e.message,
                range: e.range + delta,
            }));
        out.comments
            .extend(part.comments.into_iter().map(|c| Comment {
                range: c.range + delta,
                text: c.text,
            }));
        match error_at.and_then(|at| next_document_marker(text, base, at)) {
            Some(next) => base = next,
            None => break,
        }
    }
    out
}

/// The byte offset of the first document marker (`---` at the start of a line, followed
/// by whitespace or the end of the text) after `base`, starting with the line that
/// contains `offset`. The parser often reports an error at the marker that ends the broken
/// document, so that line counts too.
fn next_document_marker(text: &str, base: usize, offset: usize) -> Option<usize> {
    let bytes = text.as_bytes();
    let offset = offset.min(bytes.len());
    let mut line_start = bytes[..offset]
        .iter()
        .rposition(|&b| b == b'\n')
        .map_or(0, |i| i + 1);
    loop {
        let rest = &bytes[line_start..];
        let is_marker = rest.starts_with(b"---")
            && rest
                .get(3)
                .is_none_or(|b| matches!(b, b' ' | b'\t' | b'\r' | b'\n'));
        if is_marker && line_start > base {
            return Some(line_start);
        }
        line_start += rest.iter().position(|&b| b == b'\n')? + 1;
    }
}

/// Parses `text` until the end or its first syntax error.
fn parse_segment(text: &str) -> Parse {
    let offsets = ByteOffsets::new(text);
    let mut builder = Builder::new(text, &offsets);
    let mut out = Parse::default();
    let mut limit = TextSize::new(u32::try_from(text.len()).unwrap_or(u32::MAX));
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
                limit = at;
                break;
            }
        }
    }
    out.comments = comments::extract(text, &mut builder.scalars, limit);
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
    /// Ranges of every scalar seen so far, for comment extraction.
    scalars: Vec<TextRange>,
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
            scalars: Vec::new(),
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
                let end = match style {
                    SaphyrStyle::SingleQuoted | SaphyrStyle::DoubleQuoted => {
                        self.closing_quote_end(start, style)
                    }
                    _ => None,
                };
                let range = TextRange::new(start, end.unwrap_or(range.end()).max(start));
                self.scalars.push(range);
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

    /// The end of a quoted scalar starting at `start`: just past its closing quote.
    ///
    /// The parser's span for a quoted scalar runs on over trailing spaces and a trailing
    /// comment (`'x' # c` spans to the end of the line), so the end is found here. Inside
    /// single quotes `''` is an escaped quote; inside double quotes `\` escapes the next
    /// character. Returns `None` if no closing quote is found.
    fn closing_quote_end(&self, start: TextSize, style: SaphyrStyle) -> Option<TextSize> {
        let bytes = self.text.as_bytes();
        let quote = if style == SaphyrStyle::SingleQuoted {
            b'\''
        } else {
            b'"'
        };
        if bytes.get(usize::from(start)) != Some(&quote) {
            return None;
        }
        let mut pos = usize::from(start) + 1;
        loop {
            match (style, bytes.get(pos)?) {
                (SaphyrStyle::SingleQuoted, b'\'') => {
                    if bytes.get(pos + 1) == Some(&b'\'') {
                        pos += 2;
                    } else {
                        break;
                    }
                }
                (SaphyrStyle::DoubleQuoted, b'\\') => pos += 2,
                (SaphyrStyle::DoubleQuoted, b'"') => break,
                _ => pos += 1,
            }
        }
        u32::try_from(pos + 1).ok().map(TextSize::new)
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
