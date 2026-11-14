use kubelint_k8s::is_qualified_name;

use super::{key_suggestion, suggestion};

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

#[test]
fn key_suggestions() {
    let cases = [
        ("Team Name", Some("Team-Name")),
        ("Example.com/tier", Some("example.com/tier")),
        ("my_app/tier", Some("my-app/tier")),
        ("Example.COM/job type", Some("example.com/job-type")),
        ("example.com/", None),
        ("@@@/x", None),
        ("ā/x", None),
        ("x/ā", None),
    ];
    for (key, expected) in cases {
        assert_eq!(key_suggestion(key).as_deref(), expected, "{key}");
    }
}

#[test]
fn every_suggested_key_is_valid() {
    let keys = [
        "Team Name",
        "Example.com/tier",
        "a/b/c",
        "A.B.C/D_e",
        "job type",
        "-x-/-y-",
    ];
    for key in keys {
        let Some(fixed) = key_suggestion(key) else {
            continue;
        };
        assert!(is_qualified_name(&fixed).is_ok(), "{key} became {fixed}");
    }
    let long = format!("{}/{}", "A".repeat(300), "b".repeat(70));
    let fixed = key_suggestion(&long).expect("a long key has a suggestion");
    assert!(is_qualified_name(&fixed).is_ok(), "{fixed}");
}

#[test]
fn a_long_prefix_keeps_up_to_253_characters() {
    let prefix = format!("{}.Example.COM", "a".repeat(100));
    let fixed = key_suggestion(&format!("{prefix}/tier")).expect("a suggestion");
    assert_eq!(fixed, format!("{}/tier", prefix.to_lowercase()));
}
