use super::{escape_data, escape_property};

#[test]
fn escaping() {
    assert_eq!(
        escape_data("50% done\r\nnext: a, b"),
        "50%25 done%0D%0Anext: a, b"
    );
    assert_eq!(escape_property("a:b,c%\n"), "a%3Ab%2Cc%25%0A");
}
