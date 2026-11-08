use kubelint_yaml::{Node, parse};

#[test]
fn navigation_on_wrong_kind_is_empty() {
    let parse = parse("- a\n");
    let root = parse.documents[0].root.as_ref().unwrap();
    assert!(root.get("a").is_none());
    assert!(root.get_path(&["a"]).is_none());
    assert_eq!(root.entries().count(), 0);
    assert_eq!(root.get_path(&[]).map(|n| n.range), Some(root.range));
}

#[test]
fn scalar_has_no_children() {
    let parse = parse("a: b\n");
    let value = parse.documents[0].root.as_ref().unwrap().get("a").unwrap();
    assert_eq!(value.items().count(), 0);
    assert!(value.as_mapping().is_none() && value.as_sequence().is_none());
}

#[test]
fn as_bool_follows_kubernetes_decoding() {
    let parse = parse("[true, yes, Off, n, 'true', 1, null, {a: b}, ~]\n");
    let root = parse.documents[0].root.as_ref().unwrap();
    let found: Vec<_> = root.items().map(Node::as_bool).collect();
    assert_eq!(
        found,
        [
            Some(true),
            Some(true),
            Some(false),
            Some(false),
            None,
            None,
            None,
            None,
            None
        ]
    );
}
