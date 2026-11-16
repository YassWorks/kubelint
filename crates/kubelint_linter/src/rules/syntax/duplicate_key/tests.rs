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

#[test]
fn deep_nesting_does_not_overflow_a_small_stack() {
    let text = format!(
        "kind: ConfigMap\nx:\n{}{{a: 1, a: 2}}\n",
        "- ".repeat(9_999)
    );
    let out = std::thread::Builder::new()
        .stack_size(2 * 1024 * 1024)
        .spawn(move || {
            let file = SourceFile::new("deep.yaml", text);
            check_source(&DuplicateKey, &file)
        })
        .expect("the thread starts")
        .join()
        .expect("the walk stays off the call stack");
    assert_eq!(out.matches("error[KL0002]").count(), 1);
    let path = format!("x{}", "[0]".repeat(9_999));
    assert!(
        out.contains(&format!("duplicate key `a` in `{path}`")),
        "{out}"
    );
}
