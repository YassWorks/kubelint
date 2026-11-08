use super::suggestion;

#[test]
fn suggestions() {
    let cases = [
        ("hello world", Some("hello-world")),
        ("team@example.com", Some("team-example.com")),
        ("-v1-", Some("v1")),
        ("a: b", Some("a-b")),
        ("ā", None),
        ("@@", None),
    ];
    for (value, expected) in cases {
        assert_eq!(suggestion(value).as_deref(), expected, "{value}");
    }
    assert_eq!(suggestion(&"a".repeat(70)), Some("a".repeat(63)));
}
