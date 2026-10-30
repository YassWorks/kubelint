use text_size::{TextRange, TextSize};

/// A 1-based line and column.
///
/// Columns are counted in UTF-8 bytes (`utf8_col`, what terminals and SARIF-style tools
/// usually want) and in UTF-16 code units (`utf16_col`, what the language server protocol
/// uses).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LineCol {
    /// 1-based line number.
    pub line: u32,
    /// 1-based column in UTF-8 bytes.
    pub utf8_col: u32,
    /// 1-based column in UTF-16 code units.
    pub utf16_col: u32,
}

/// Maps byte offsets to lines and columns and back.
///
/// Only `\n` ends a line. In a CRLF file the `\r` is the last byte of its line, which
/// keeps offsets simple and matches how editors count columns.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LineIndex {
    /// Byte offset of the start of every line. Always begins with 0.
    line_starts: Vec<TextSize>,
    len: TextSize,
}

impl LineIndex {
    /// Indexes `text`.
    ///
    /// ```
    /// use kubelint_source::LineIndex;
    ///
    /// let index = LineIndex::new("a\nb\n");
    /// assert_eq!(index.line_count(), 3);
    /// ```
    ///
    /// # Panics
    ///
    /// Panics if `text` is longer than 4 GiB, which `TextSize` cannot represent.
    pub fn new(text: &str) -> Self {
        let mut line_starts = vec![TextSize::new(0)];
        for (i, b) in text.bytes().enumerate() {
            if b == b'\n' {
                line_starts.push(to_size(i + 1));
            }
        }
        Self {
            line_starts,
            len: to_size(text.len()),
        }
    }

    /// Number of lines. An empty file and a file without a trailing newline both count
    /// their last (possibly empty) line, so `"a\n"` has 2 lines.
    pub fn line_count(&self) -> usize {
        self.line_starts.len()
    }

    /// Converts a byte offset to a 1-based line and column.
    ///
    /// `text` must be the text this index was built from. An offset equal to the text
    /// length (end of file) is valid. Offsets past the end are clamped to it. An offset
    /// inside a multi-byte character is treated as the start of that character.
    ///
    /// ```
    /// use kubelint_source::{LineIndex, TextSize};
    ///
    /// let text = "a: é\nb: 1\n";
    /// let index = LineIndex::new(text);
    /// let pos = index.line_col(text, TextSize::new(6));
    /// assert_eq!((pos.line, pos.utf8_col, pos.utf16_col), (2, 1, 1));
    /// ```
    pub fn line_col(&self, text: &str, offset: TextSize) -> LineCol {
        let offset = offset.min(self.len);
        let line = self.line_starts.partition_point(|&s| s <= offset) - 1;
        let start = usize::from(self.line_starts[line]);
        let mut end = usize::from(offset);
        while !text.is_char_boundary(end) {
            end -= 1;
        }
        let prefix = &text[start..end];
        LineCol {
            line: to_u32(line + 1),
            utf8_col: to_u32(prefix.len() + 1),
            utf16_col: to_u32(prefix.encode_utf16().count() + 1),
        }
    }

    /// The byte range of a 1-based line, without its line terminator (`\n` or `\r\n`).
    /// Returns `None` if the line does not exist.
    ///
    /// ```
    /// use kubelint_source::{LineIndex, TextRange};
    ///
    /// let text = "ab\r\ncd";
    /// let index = LineIndex::new(text);
    /// assert_eq!(index.line_range(text, 1), Some(TextRange::new(0.into(), 2.into())));
    /// assert_eq!(index.line_range(text, 3), None);
    /// ```
    pub fn line_range(&self, text: &str, line: u32) -> Option<TextRange> {
        let i = usize::try_from(line.checked_sub(1)?).ok()?;
        let start = *self.line_starts.get(i)?;
        let mut end = self
            .line_starts
            .get(i + 1)
            .map_or(self.len, |&next| next - TextSize::new(1));
        if end > start && text.as_bytes()[usize::from(end) - 1] == b'\r' {
            end -= TextSize::new(1);
        }
        Some(TextRange::new(start, end))
    }

    /// Converts a 1-based line and a 1-based UTF-8 byte column to a byte offset.
    /// Returns `None` if the line does not exist or the column lies past the end of the
    /// line (the position just before the `\n` is allowed, for end-of-line positions).
    ///
    /// ```
    /// use kubelint_source::{LineIndex, TextSize};
    ///
    /// let index = LineIndex::new("ab\ncd\n");
    /// assert_eq!(index.offset(2, 2), Some(TextSize::new(4)));
    /// assert_eq!(index.offset(2, 9), None);
    /// ```
    pub fn offset(&self, line: u32, utf8_col: u32) -> Option<TextSize> {
        let i = usize::try_from(line.checked_sub(1)?).ok()?;
        let start = *self.line_starts.get(i)?;
        let end = self
            .line_starts
            .get(i + 1)
            .map_or(self.len, |&next| next - TextSize::new(1));
        let offset = start.checked_add(TextSize::new(utf8_col.checked_sub(1)?))?;
        (offset <= end).then_some(offset)
    }
}

fn to_size(n: usize) -> TextSize {
    TextSize::try_from(n).expect("file is smaller than 4 GiB")
}

fn to_u32(n: usize) -> u32 {
    u32::try_from(n).expect("line and column numbers fit in u32 because offsets do")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pos(text: &str, offset: u32) -> (u32, u32, u32) {
        let p = LineIndex::new(text).line_col(text, TextSize::new(offset));
        (p.line, p.utf8_col, p.utf16_col)
    }

    #[test]
    fn ascii() {
        let t = "kind: Pod\nname: x\n";
        assert_eq!(pos(t, 0), (1, 1, 1));
        assert_eq!(pos(t, 5), (1, 6, 6));
        assert_eq!(pos(t, 10), (2, 1, 1));
        assert_eq!(pos(t, 16), (2, 7, 7));
    }

    #[test]
    fn non_ascii_columns_differ_by_unit() {
        // "é" is 2 bytes / 1 unit, "😀" is 4 bytes / 2 units, "世" is 3 bytes / 1 unit.
        let t = "é😀世x\ny";
        assert_eq!(pos(t, 0), (1, 1, 1));
        assert_eq!(pos(t, 2), (1, 3, 2));
        assert_eq!(pos(t, 6), (1, 7, 4));
        assert_eq!(pos(t, 9), (1, 10, 5));
        assert_eq!(pos(t, 11), (2, 1, 1));
    }

    #[test]
    fn offset_inside_a_character_snaps_to_its_start() {
        assert_eq!(pos("é", 1), (1, 1, 1));
    }

    #[test]
    fn crlf() {
        let t = "a\r\nb\r\n";
        assert_eq!(pos(t, 1), (1, 2, 2));
        assert_eq!(pos(t, 3), (2, 1, 1));
        assert_eq!(pos(t, 6), (3, 1, 1));
        let index = LineIndex::new(t);
        assert_eq!(
            index.line_range(t, 1),
            Some(TextRange::new(0.into(), 1.into()))
        );
        assert_eq!(
            index.line_range(t, 2),
            Some(TextRange::new(3.into(), 4.into()))
        );
    }

    #[test]
    fn empty_file() {
        let index = LineIndex::new("");
        assert_eq!(index.line_count(), 1);
        assert_eq!(pos("", 0), (1, 1, 1));
        assert_eq!(pos("", 10), (1, 1, 1));
        assert_eq!(index.line_range("", 1), Some(TextRange::empty(0.into())));
        assert_eq!(index.offset(1, 1), Some(TextSize::new(0)));
    }

    #[test]
    fn end_of_file() {
        assert_eq!(pos("ab", 2), (1, 3, 3));
        assert_eq!(pos("ab\n", 3), (2, 1, 1));
        assert_eq!(pos("ab", 99), (1, 3, 3));
    }

    #[test]
    fn offset_round_trips() {
        let t = "é: 1\nlong line\n\nlast";
        let index = LineIndex::new(t);
        for (i, _) in t.char_indices() {
            let off = TextSize::try_from(i).unwrap();
            let p = index.line_col(t, off);
            assert_eq!(index.offset(p.line, p.utf8_col), Some(off));
        }
    }

    #[test]
    fn offset_rejects_bad_input() {
        let index = LineIndex::new("ab\ncd");
        assert_eq!(index.offset(0, 1), None);
        assert_eq!(index.offset(1, 0), None);
        assert_eq!(index.offset(3, 1), None);
        assert_eq!(index.offset(1, 4), None);
        assert_eq!(index.offset(1, 3), Some(TextSize::new(2)));
    }
}
