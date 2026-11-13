//! Byte-level helpers for quoted scalars.

/// Walks the quoted scalar whose opening quote is at `start`: returns the offset just
/// past its closing quote, and the offsets of its `\'` escapes. `saphyr-parser` reports a
/// quoted scalar's span running on over trailing spaces and a trailing comment, so the
/// builder finds the closing quote itself. libyaml and `go-yaml` read `\'` as `'`, but
/// `saphyr-parser` rejects it.
pub(crate) fn scan(text: &str, start: usize) -> Option<(usize, Vec<usize>)> {
    let bytes = text.as_bytes();
    let quote = *bytes.get(start).filter(|b| matches!(b, b'\'' | b'"'))?;
    let mut quote_escapes = Vec::new();
    let mut pos = start + 1;
    loop {
        match (quote, *bytes.get(pos)?) {
            (b'\'', b'\'') if bytes.get(pos + 1) == Some(&b'\'') => pos += 2,
            (b'"', b'\\') => {
                if bytes.get(pos + 1) == Some(&b'\'') {
                    quote_escapes.push(pos);
                }
                pos += 2;
            }
            (_, byte) if byte == quote => return Some((pos + 1, quote_escapes)),
            _ => pos += 1,
        }
    }
}

/// Turns back into `'` each BEL character of `value` that came from a rewritten `\'`.
/// `raw` is the scalar's source text starting at byte `offset`, quotes included. Other
/// sources of BEL (`\a`, `\x07`, `\u0007`, `\U00000007`) keep it.
pub(crate) fn restore(value: &str, raw: &str, offset: usize, rewritten: &[usize]) -> String {
    let bytes = raw.as_bytes();
    let mut from_quote = Vec::new();
    let mut pos = 0;
    while pos < bytes.len() {
        match bytes[pos] {
            b'\\' => {
                let escape = &raw[pos + 1..];
                if escape.starts_with('a') {
                    from_quote.push(rewritten.binary_search(&(offset + pos)).is_ok());
                } else if ["x07", "u0007", "U00000007"]
                    .iter()
                    .any(|code| escape.starts_with(code))
                {
                    from_quote.push(false);
                }
                pos += 2;
            }
            0x07 => {
                from_quote.push(false);
                pos += 1;
            }
            _ => pos += 1,
        }
    }
    let mut flags = from_quote.into_iter();
    value
        .chars()
        .map(|c| match c {
            '\u{7}' if flags.next() == Some(true) => '\'',
            c => c,
        })
        .collect()
}
