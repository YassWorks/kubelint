use kubelint_source::SourceFile;

use super::{DuplicateKey, find_duplicates, push_key};
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
    // With the root and the inner mapping, 10,000 nested collections: the most kubectl accepts.
    let text = format!("x:\n{}{{a: 1, a: 2}}\n", "- ".repeat(9_998));
    // Parsing and dropping the tree recurse per level, so they get a large stack and only
    // the walk runs on a small one.
    let found = std::thread::Builder::new()
        .stack_size(64 * 1024 * 1024)
        .spawn(move || {
            let parse = kubelint_yaml::parse(&text);
            let root = parse.documents[0].root.as_ref().expect("a root");
            let found = std::thread::scope(|scope| {
                std::thread::Builder::new()
                    .stack_size(256 * 1024)
                    .spawn_scoped(scope, || find_duplicates(root))
                    .expect("the thread starts")
                    .join()
                    .expect("the walk stays off the call stack")
            });
            found
                .into_iter()
                .map(|d| (d.key, d.path))
                .collect::<Vec<_>>()
        })
        .expect("the thread starts")
        .join()
        .expect("parsing fits in a large stack");
    let path = format!("x{}", "[0]".repeat(9_998));
    assert_eq!(found, [("a".to_owned(), path)]);
}
