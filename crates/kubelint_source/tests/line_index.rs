use kubelint_source::{LineIndex, TextRange, TextSize};

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
