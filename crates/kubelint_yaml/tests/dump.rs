use kubelint_source::TextRange;
use kubelint_yaml::{Document, Entry, Node, NodeKind, Scalar, ScalarStyle};

fn range(start: u32, end: u32) -> TextRange {
    TextRange::new(start.into(), end.into())
}

fn scalar(value: &str, style: ScalarStyle, r: TextRange) -> Node {
    Node {
        kind: NodeKind::Scalar(Scalar {
            value: value.to_owned(),
            style,
        }),
        range: r,
    }
}

#[test]
fn empty_document() {
    let doc = Document {
        root: None,
        range: range(0, 0),
    };
    assert_eq!(doc.dump(), "document 0..0\n  (empty)\n");
}

#[test]
fn dump_of_nested_tree_with_non_ascii() {
    let doc = Document {
        root: Some(Node {
            kind: NodeKind::Mapping(vec![
                Entry {
                    key: scalar("name", ScalarStyle::Plain, range(0, 4)),
                    value: scalar("é", ScalarStyle::DoubleQuoted, range(6, 9)),
                },
                Entry {
                    key: scalar("ports", ScalarStyle::Plain, range(10, 15)),
                    value: Node {
                        kind: NodeKind::Sequence(vec![scalar(
                            "80",
                            ScalarStyle::Plain,
                            range(18, 20),
                        )]),
                        range: range(17, 21),
                    },
                },
                Entry {
                    key: scalar("none", ScalarStyle::Plain, range(22, 26)),
                    value: Node {
                        kind: NodeKind::Null,
                        range: range(27, 27),
                    },
                },
            ]),
            range: range(0, 27),
        }),
        range: range(0, 28),
    };
    insta::assert_snapshot!(doc.dump());
}
