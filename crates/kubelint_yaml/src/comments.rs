//! `saphyr-parser` does not report comments, so they are found by scanning the text: a
//! `#` that starts a line or follows a space or tab, and is not inside any scalar's range,
//! starts a comment that runs to the end of the line. YAML never starts a comment right
//! after another character (`a#b` is one plain scalar).

use kubelint_source::{TextRange, TextSize};

/// A `# ...` comment. Suppression comments (`# kubelint-disable-...`) are read from these.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Comment {
    /// From the `#` to the end of the line, without the line break.
    pub range: TextRange,
    /// The comment text after the `#`, as written (leading spaces kept).
    pub text: String,
}

/// Finds the comments in `text[..limit]`.
///
/// `scalars` are the ranges of every scalar the parser reported, in any order. `limit`
/// is where reliable scalar information ends (the first parse error, or the end of the
/// text): past it a `#` could be inside a scalar the parser never reported.
pub(crate) fn extract(text: &str, scalars: &mut [TextRange], limit: TextSize) -> Vec<Comment> {
    scalars.sort_unstable_by_key(|r| r.start());
    let bytes = text.as_bytes();
    let limit = usize::from(limit).min(bytes.len());
    let mut comments = Vec::new();
    let mut pos = 0;
    while pos < limit {
        let line_end = bytes[pos..]
            .iter()
            .position(|&b| b == b'\n')
            .map_or(bytes.len(), |i| pos + i);
        if let Some(hash) = find_comment_start(bytes, pos, line_end.min(limit), scalars) {
            let mut end = line_end;
            if end > hash && bytes[end - 1] == b'\r' {
                end -= 1;
            }
            comments.push(Comment {
                range: TextRange::new(to_size(hash), to_size(end)),
                text: text[hash + 1..end].to_owned(),
            });
        }
        pos = line_end + 1;
    }
    comments
}

/// The first `#` in `bytes[start..end]` that starts a comment.
fn find_comment_start(
    bytes: &[u8],
    start: usize,
    end: usize,
    scalars: &[TextRange],
) -> Option<usize> {
    (start..end).find(|&i| {
        bytes[i] == b'#'
            && (i == start || matches!(bytes[i - 1], b' ' | b'\t'))
            && !inside_scalar(scalars, i)
    })
}

/// Whether byte offset `at` lies inside one of the (sorted) scalar ranges.
fn inside_scalar(scalars: &[TextRange], at: usize) -> bool {
    let at = to_size(at);
    // The last scalar starting at or before `at` is the only one that can contain it,
    // because scalars never overlap.
    let idx = scalars.partition_point(|r| r.start() <= at);
    idx > 0 && scalars[idx - 1].contains(at)
}

fn to_size(offset: usize) -> TextSize {
    TextSize::new(u32::try_from(offset).unwrap_or(u32::MAX))
}
