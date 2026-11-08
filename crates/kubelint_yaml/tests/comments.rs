//! Comment extraction. Every input contains non-ASCII text before the interesting
//! comments, so ranges are checked as byte offsets.

use std::fmt::Write as _;

use kubelint_yaml::parse;

/// One line per comment: its byte range, then the source slice it covers.
fn comments(text: &str) -> String {
    let mut out = String::new();
    for c in parse(text).comments {
        assert_eq!(
            c.text,
            text[c.range][1..],
            "text is the range minus the `#`"
        );
        let _ = writeln!(out, "{:?} {:?}", c.range, &text[c.range]);
    }
    out
}

#[test]
fn whole_line_and_trailing_comments() {
    let text = include_str!("fixtures/comments/whole_line_and_trailing_comments.yaml");
    insta::assert_snapshot!(comments(text));
}

#[test]
fn hash_inside_quoted_scalars_is_not_a_comment() {
    let text = include_str!("fixtures/comments/hash_inside_quoted_scalars_is_not_a_comment.yaml");
    insta::assert_snapshot!(comments(text));
}

#[test]
fn hash_inside_block_scalars_is_not_a_comment() {
    let text = include_str!("fixtures/comments/hash_inside_block_scalars_is_not_a_comment.yaml");
    insta::assert_snapshot!(comments(text));
}

#[test]
fn hash_without_preceding_space_is_part_of_the_scalar() {
    let text = "é: a#b\nurl: http://x/#frag\nc: [d#e] #f\n";
    insta::assert_snapshot!(comments(text));
}

#[test]
fn comments_in_flow_collections_and_after_document_markers() {
    let text = include_str!(
        "fixtures/comments/comments_in_flow_collections_and_after_document_markers.yaml"
    );
    insta::assert_snapshot!(comments(text));
}

#[test]
fn tab_before_hash_and_crlf_line_endings() {
    let text = "é: 1\t# tab\r\nb: 2 # crlf\r\n";
    insta::assert_snapshot!(comments(text));
}

#[test]
fn comments_after_a_syntax_error_are_not_reported() {
    let text =
        include_str!("fixtures/comments/comments_after_a_syntax_error_are_not_reported.yaml");
    let parse = parse(text);
    assert_eq!(parse.errors.len(), 1);
    insta::assert_snapshot!(comments(text));
}

#[test]
fn empty_and_comment_only_files() {
    assert_eq!(comments(""), "");
    insta::assert_snapshot!(comments("# ünï only\n#\n"));
}

#[test]
fn comments_resume_after_the_next_document_marker() {
    // The error is reported at the `---` line, so parsing resumes there.
    let text =
        include_str!("fixtures/comments/comments_resume_after_the_next_document_marker.yaml");
    insta::assert_snapshot!(comments(text));
}
