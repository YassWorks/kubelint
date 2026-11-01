//! Comment extraction. Every input contains non-ASCII text before the interesting
//! comments, so ranges are checked as byte offsets.

use std::fmt::Write as _;

use kubelint_yaml::parse;

/// One line per comment: its byte range, then the source slice it covers.
fn comments(text: &str) -> String {
    let mut out = String::new();
    for c in parse(text).comments {
        let (start, end) = (u32::from(c.range.start()), u32::from(c.range.end()));
        let _ = writeln!(out, "{start}..{end} {:?}", &text[c.range]);
        assert_eq!(
            c.text,
            text[c.range][1..],
            "text is the range minus the `#`"
        );
    }
    out
}

#[test]
fn whole_line_and_trailing_comments() {
    let text = "# ünï\né: 1 # trailing\n  # indented\nb: 2\n";
    insta::assert_snapshot!(comments(text));
}

#[test]
fn hash_inside_quoted_scalars_is_not_a_comment() {
    let text = "é: \"a #b\"\nc: 'd #e' # real\nf: \"g\n  #h\"\n";
    insta::assert_snapshot!(comments(text));
}

#[test]
fn hash_inside_block_scalars_is_not_a_comment() {
    let text = "é: | # header\n  #!/bin/sh\n  echo hi # not a comment\n\n  # still text\nf: > # folded\n  x # y\n# after\n";
    insta::assert_snapshot!(comments(text));
}

#[test]
fn hash_without_preceding_space_is_part_of_the_scalar() {
    let text = "é: a#b\nurl: http://x/#frag\nc: [d#e] #f\n";
    insta::assert_snapshot!(comments(text));
}

#[test]
fn comments_in_flow_collections_and_after_document_markers() {
    let text = "--- # first\né: [1, # one\n  2]\n...\n# between\n---\nb: {c: d} # map\n";
    insta::assert_snapshot!(comments(text));
}

#[test]
fn tab_before_hash_and_crlf_line_endings() {
    let text = "é: 1\t# tab\r\nb: 2 # crlf\r\n";
    insta::assert_snapshot!(comments(text));
}

#[test]
fn comments_after_a_syntax_error_are_not_reported() {
    let text = "é: 1 # kept\n---\na: [b # lost?\nc: \"unterminated # dropped\n";
    let parse = parse(text);
    assert_eq!(parse.errors.len(), 1);
    insta::assert_snapshot!(comments(text));
}

#[test]
fn empty_and_comment_only_files() {
    assert_eq!(comments(""), "");
    insta::assert_snapshot!(comments("# ünï only\n#\n"));
}
