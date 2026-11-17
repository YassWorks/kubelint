//! Tests of merge key (`<<`) expansion. Every input has non-ASCII text before the
//! interesting nodes, so character-to-byte conversion is exercised.

use kubelint_yaml::{Document, parse};

fn dump(text: &str) -> String {
    let parse = parse(text);
    let errors = parse.errors.iter().map(|e| {
        format!(
            "error {:?} at {} limit {}\n",
            e.message,
            u32::from(e.range.start()),
            e.limit
        )
    });
    parse
        .documents
        .iter()
        .map(Document::dump)
        .chain(errors)
        .collect()
}

#[test]
fn a_merge_writes_its_entries_where_it_stands() {
    insta::assert_snapshot!(dump(include_str!("fixtures/merge/overrides.yaml")));
}

#[test]
fn the_earlier_source_of_a_sequence_wins() {
    insta::assert_snapshot!(dump(include_str!("fixtures/merge/sources.yaml")));
}

#[test]
fn an_anchor_that_merges_is_already_expanded() {
    insta::assert_snapshot!(dump(include_str!("fixtures/merge/nested.yaml")));
}

#[test]
fn only_a_plain_or_merge_tagged_key_merges() {
    insta::assert_snapshot!(dump(include_str!("fixtures/merge/keys.yaml")));
}

#[test]
fn merges_in_flow_collections() {
    insta::assert_snapshot!(dump(include_str!("fixtures/merge/flow.yaml")));
}

#[test]
fn a_merge_value_that_is_not_mappings_stops_its_document() {
    insta::assert_snapshot!(dump(include_str!("fixtures/merge/invalid_values.yaml")));
}

#[test]
fn a_merge_error_is_not_a_limit() {
    let parse = parse(include_str!("fixtures/merge/invalid_values.yaml"));
    assert_eq!(parse.errors.len(), 7);
    assert!(parse.errors.iter().all(|e| !e.limit));
}

#[test]
fn merged_entries_share_an_origin_per_source() {
    let parse = parse(include_str!("fixtures/merge/origins.yaml"));
    let root = parse.documents[0].root.as_ref().expect("a root");
    let seen: Vec<_> = root
        .entries()
        .map(|e| format!("{}:{}", e.key_str().unwrap_or("?"), e.origin))
        .collect();
    insta::assert_debug_snapshot!(seen);
}
