use kubelint_source::SourceFile;

use super::{DuplicateKey, push_key};
use crate::testing::{check_source, read};

#[test]
fn a_duplicate_repeated_by_an_alias_is_reported_once() {
    let file = SourceFile::new(
        "alias.yaml",
        read("resources/test/duplicate_key/alias.yaml"),
    );
    let out = check_source(&DuplicateKey, &file);
    assert_eq!(out.matches("error[KL0002]").count(), 1, "{out}");
}

#[test]
fn paths_quote_keys_that_are_not_simple() {
    let mut path = String::new();
    push_key(&mut path, "metadata");
    push_key(&mut path, "annotations");
    push_key(&mut path, "example.com/owner");
    assert_eq!(path, r#"metadata.annotations["example.com/owner"]"#);
}
