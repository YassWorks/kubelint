//! Tests of the alias and nesting limits. Every input has non-ASCII text, so
//! character-to-byte conversion stays exercised.

use std::thread;

use kubelint_yaml::{Document, parse};

const ALIASES: &str = "aliases expand to too many nodes";
/// The node counts of the budget tests include this header.
const CONFIG_MAP: &str = include_str!("fixtures/parse/config_map_header.yaml");
const DEPTH: &str = "nesting is deeper than 10000 levels";

/// Runs `check` on a thread with room to build and drop a tree 10,000 levels deep.
fn deep(check: impl FnOnce() + Send + 'static) {
    thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(check)
        .expect("a thread starts")
        .join()
        .expect("the check passes");
}

/// The error messages and document count of `text`.
fn outcome(text: &str) -> (Vec<String>, usize) {
    let parse = parse(text);
    let errors = parse.errors.into_iter().map(|e| e.message).collect();
    (errors, parse.documents.len())
}

/// `levels` nested block sequences under the root mapping, so the document is
/// `levels + 1` collections deep.
fn sequences(levels: usize) -> String {
    format!("é:\n{}x\n", "- ".repeat(levels))
}

/// `levels` collections nested under the root mapping: sequences and a last mapping.
fn collections(levels: usize) -> String {
    format!("é:\n{}k: x\n", "- ".repeat(levels - 1))
}

/// An anchor mapping holding `depth` nested sequences, merged by a mapping two levels down, as
/// `<<: *a` or as `<<: [*a]`.
fn merged_anchor(depth: usize, sequence: bool) -> String {
    let merge = if sequence { "[*a]" } else { "*a" };
    let seqs = "- ".repeat(depth);
    format!("x: &a\n  k:\n    {seqs}b\nz:\n  y:\n    <<: {merge}\n")
}

/// `uses` items that each merge an anchored mapping of 100 keys under a `!!str` key,
/// which is not a merge key.
fn tagged_merges(uses: usize) -> String {
    let keys: Vec<_> = (0..100).map(|i| format!("k{i}: é")).collect();
    let items = "- !!str <<: *a\n".repeat(uses);
    format!("{CONFIG_MAP}x: &a {{{}}}\ny:\n{items}", keys.join(", "))
}

/// An anchored mapping of 400 keys and a mapping that merges it `uses` times.
fn merges(uses: usize) -> String {
    let keys: Vec<_> = (0..400).map(|i| format!("k{i}: é")).collect();
    let keys = keys.join(", ");
    let uses = vec!["*a"; uses].join(", ");
    format!("{CONFIG_MAP}x: &a {{{keys}}}\ny: {{<<: [{uses}]}}\n")
}

/// An anchored sequence of 500 scalars and `uses` aliases to it.
fn aliases(uses: usize) -> String {
    let items = vec!["é"; 500].join(", ");
    let uses = vec!["*a"; uses].join(", ");
    format!("{CONFIG_MAP}x: &a [{items}]\ny: [{uses}]\n")
}

#[test]
fn anchors_within_the_alias_budget_are_expanded() {
    let parse = parse(include_str!(
        "fixtures/parse/anchors_within_the_alias_budget.yaml"
    ));
    assert!(parse.errors.is_empty(), "{:?}", parse.errors);
    insta::assert_snapshot!(
        parse
            .documents
            .iter()
            .map(Document::dump)
            .collect::<String>()
    );
}

#[test]
fn an_alias_bomb_stops_its_document_and_the_next_one_is_parsed() {
    let parse = parse(include_str!(
        "fixtures/parse/alias_bomb_then_a_valid_document.yaml"
    ));
    assert!(parse.errors.iter().all(|e| e.limit));
    let errors: Vec<_> = parse
        .errors
        .iter()
        .map(|e| (&e.message[..], e.range))
        .collect();
    let documents: String = parse.documents.iter().map(Document::dump).collect();
    insta::assert_snapshot!(format!("{errors:?}\n{documents}"));
}

#[test]
fn the_alias_budget_is_where_kubectl_puts_it() {
    // kubectl 1.34 to 1.37 accept 126 uses of this anchor and refuse 127.
    assert_eq!(outcome(&aliases(126)), (vec![], 1));
    assert_eq!(outcome(&aliases(127)), (vec![ALIASES.into()], 0));
}

#[test]
fn merge_keys_are_not_visited_as_the_decoder_does_not_visit_them() {
    // kubectl 1.34 to 1.37 accept 114 merges of this anchor and refuse 115.
    assert_eq!(outcome(&merges(114)), (vec![], 1));
    assert_eq!(outcome(&merges(115)), (vec![ALIASES.into()], 0));
}

#[test]
fn nesting_up_to_the_limit_is_parsed() {
    deep(|| {
        assert_eq!(outcome(&sequences(9999)), (vec![], 1));
        assert_eq!(outcome(&collections(9999)), (vec![], 1));
    });
}

#[test]
fn nesting_past_the_limit_stops_the_document() {
    deep(|| {
        assert_eq!(outcome(&sequences(10_000)), (vec![DEPTH.into()], 0));
        assert_eq!(outcome(&collections(10_000)), (vec![DEPTH.into()], 0));
        assert_eq!(outcome(&sequences(200_000)), (vec![DEPTH.into()], 0));
    });
}

#[test]
fn a_merge_adds_no_nesting_level() {
    // kubectl 1.34 and 1.37 accept an anchor with 9,997 nested sequences merged two levels
    // down, as a value or in a sequence, and refuse 9,998.
    deep(|| {
        for sequence in [false, true] {
            assert_eq!(outcome(&merged_anchor(9997, sequence)), (vec![], 1));
            assert_eq!(
                outcome(&merged_anchor(9998, sequence)),
                (vec![DEPTH.into()], 0)
            );
        }
    });
}

#[test]
fn only_a_plain_or_merge_tagged_key_is_a_merge_key() {
    // kubectl 1.37 accepts 2,062 of these items and refuses 2,063.
    assert_eq!(outcome(&tagged_merges(2062)), (vec![], 1));
    assert_eq!(outcome(&tagged_merges(2063)), (vec![ALIASES.into()], 0));
}

#[test]
fn an_alias_counts_the_nesting_of_what_it_points_to() {
    // The anchor is 9,999 levels deep. kubectl refuses the alias below a sequence.
    let anchored = format!("é: &a\n{}x\n", "- ".repeat(9999));
    deep(move || {
        assert_eq!(outcome(&format!("{anchored}y: *a\n")), (vec![], 1));
        assert_eq!(
            outcome(&format!("{anchored}y:\n- *a\n")),
            (vec![DEPTH.into()], 0)
        );
    });
}

#[test]
fn documents_after_a_limit_are_parsed() {
    deep(|| {
        let text = format!(
            "{}---\nb: 1\n---\n{}---\nc: 2\n",
            sequences(10_000),
            aliases(127)
        );
        let parse = parse(&text);
        assert_eq!(parse.documents.len(), 2);
        let messages: Vec<_> = parse.errors.iter().map(|e| &e.message[..]).collect();
        assert_eq!(messages, [DEPTH, ALIASES]);
    });
}

#[test]
fn the_error_points_at_the_node_that_is_over_the_limit() {
    let text = aliases(127);
    let at = usize::from(parse(&text).errors[0].range.start());
    assert!(text[at..].starts_with("*a"), "error at {at}");
    deep(|| {
        let parse = parse(&sequences(10_000));
        let at = usize::from(parse.errors[0].range.start());
        // The 10,001st collection is the last dash of the line.
        assert_eq!(at, "é:\n".len() + 2 * 9999);
    });
}

#[test]
fn a_syntax_error_is_not_a_limit() {
    assert!(!parse("é: [1\n").errors[0].limit);
}
