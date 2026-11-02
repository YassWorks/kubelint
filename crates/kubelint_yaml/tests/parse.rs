//! Snapshot tests of the builder. Every input contains non-ASCII text before the
//! interesting nodes, so character-to-byte conversion is exercised.

use std::fmt::Write as _;

use kubelint_yaml::parse;

fn dump(text: &str) -> String {
    let parse = parse(text);
    let mut out = String::new();
    for doc in &parse.documents {
        out.push_str(&doc.dump());
    }
    for err in &parse.errors {
        let _ = writeln!(
            out,
            "error {:?} at {}",
            err.message,
            u32::from(err.range.start())
        );
    }
    out
}

#[test]
fn block_mapping_with_non_ascii() {
    insta::assert_snapshot!(dump("name: \"é\"\nkind: Pod\nnote: ünï\n"));
}

#[test]
fn nested_block_collections() {
    insta::assert_snapshot!(dump("é: 1\nitems:\n- x\n- b: 1\n  c: 2\n"));
}

#[test]
fn flow_collections() {
    insta::assert_snapshot!(dump("é: [80, 9]\nm: {d: e, f: []}\n"));
}

#[test]
fn block_scalars() {
    insta::assert_snapshot!(dump("é: |\n  a\n  b\nf: >\n  c\n  d\n"));
}

#[test]
fn quoted_scalars() {
    insta::assert_snapshot!(dump("é: 'one'\nb: \"t\\u00e9\"\n"));
}

#[test]
fn quoted_scalars_end_at_their_closing_quote() {
    insta::assert_snapshot!(dump(
        "é: 'it''s' # c\nb: \"q\\\"#\"  # d\nc: [ 'x' , \"y\" ]\n"
    ));
}

#[test]
fn missing_values_are_null() {
    insta::assert_snapshot!(dump("é:\nb: ~\nc:\n- \n"));
}

#[test]
fn anchors_and_aliases() {
    insta::assert_snapshot!(dump("é: &a x\nb: *a\nc: &m {k: v}\nd: *m\n"));
}

#[test]
fn multiple_documents_and_empty_ones() {
    insta::assert_snapshot!(dump("# é\n---\na: 1\n---\n---\nb: 2\n"));
}

#[test]
fn empty_input_has_no_documents() {
    assert!(parse("").documents.is_empty());
}

#[test]
fn error_keeps_earlier_documents() {
    insta::assert_snapshot!(dump("é: 1\n---\na: 1\n- bad\n"));
}

#[test]
fn error_position_is_a_byte_offset() {
    let parse = parse("é: [1\n");
    assert_eq!(parse.errors.len(), 1);
    let at = u32::from(parse.errors[0].range.start());
    // The error is after the 2-byte `é`, so a character index would be smaller.
    assert!(at >= 7, "error at {at}");
}

#[test]
fn error_resumes_at_the_next_document_marker() {
    // The second and fourth documents are broken; `----` and `---x` are not markers, so
    // the fourth document runs until `--- # é` and recovery resumes there.
    insta::assert_snapshot!(dump(
        "a: 1\n---\nb: [1\nc: 2\n---\nd: é\n---\ne: 1\n- bad\n----\n---x\n--- # é\nf: 1\n"
    ));
}

#[test]
fn error_in_the_last_document_stops_parsing() {
    let parse = parse("a: 1\n---\nb: [1\nc: 2\n");
    assert_eq!(parse.documents.len(), 1);
    assert_eq!(parse.errors.len(), 1);
}

#[test]
fn every_broken_document_reports_one_error() {
    let parse = parse("a: [\n---\nb: ]\n---\nc: 1\n");
    assert_eq!(parse.errors.len(), 2);
    assert_eq!(parse.documents.len(), 1);
    let starts: Vec<u32> = parse
        .errors
        .iter()
        .map(|e| e.range.start().into())
        .collect();
    assert!(starts.is_sorted() && starts[1] >= 5, "{starts:?}");
}
