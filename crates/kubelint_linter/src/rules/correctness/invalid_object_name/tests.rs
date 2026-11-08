use super::{NameFormat, suggestion};

#[test]
fn suggestions() {
    let cases = [
        ("My_App", NameFormat::Subdomain, Some("my-app")),
        ("web.", NameFormat::Subdomain, Some("web")),
        ("a__.b", NameFormat::Subdomain, Some("a.b")),
        ("api.v2", NameFormat::Label, Some("api-v2")),
        ("Web Server", NameFormat::Dns1035, Some("web-server")),
        ("1web", NameFormat::Dns1035, None),
        ("café", NameFormat::Dns1035, None),
        ("___", NameFormat::Subdomain, None),
        ("a/b", NameFormat::PathSegment, None),
    ];
    for (name, format, expected) in cases {
        assert_eq!(suggestion(name, format).as_deref(), expected, "{name}");
    }
    let long = "a".repeat(70);
    assert_eq!(suggestion(&long, NameFormat::Label), Some("a".repeat(63)));
}
