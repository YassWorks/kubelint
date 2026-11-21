use kubelint_schema::KubernetesVersion;

use super::{InvalidObjectName, NameFormat, NameRule, mask_trailing_dash, suggestion};
use crate::testing::{check_fixture, check_fixture_at};

fn version(text: &str) -> KubernetesVersion {
    KubernetesVersion::parse(text).expect("a supported version")
}

#[test]
fn service_and_replication_controller_names_follow_the_version() {
    for version in [version("1.34"), version("1.35")] {
        insta::assert_snapshot!(
            format!("KL1008@{version}"),
            check_fixture_at(&InvalidObjectName, version)
        );
    }
    assert_eq!(
        check_fixture_at(&InvalidObjectName, version("1.36")),
        check_fixture(&InvalidObjectName)
    );
}

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

#[test]
fn trailing_dash_is_masked() {
    let cases = [
        ("a-", "a"),
        ("ab-", "aa"),
        ("a.-", "aa"),
        ("-", "-"),
        ("a", "a"),
        ("é-", "é-"),
    ];
    for (input, expected) in cases {
        assert_eq!(mask_trailing_dash(input), expected, "{input}");
    }
}

#[test]
fn prefixes() {
    let prefix = |format, generated, max_len| NameRule {
        prefix: true,
        generated,
        max_len,
        ..NameRule::of(format)
    };
    let label = prefix(NameFormat::Label, true, None);
    let subdomain = prefix(NameFormat::Subdomain, true, None);
    let cron = prefix(NameFormat::Subdomain, true, Some(47));
    let with_name = prefix(NameFormat::Subdomain, false, None);
    let cases = [
        (label, "web-", true),
        (label, "a.b-", false),
        (label, "-", false),
        (subdomain, "a.-", false),
        (subdomain, "a.b-", true),
        (with_name, "a.-", true),
        (with_name, "Bad_Gen-", false),
        (label, &format!("{}-", "a".repeat(63)), true),
        (label, &format!("{}-", "a".repeat(64)), false),
        (cron, &"a".repeat(47), true),
        (cron, &"a".repeat(48), false),
        (cron, &format!("{}-", "a".repeat(47)), false),
        (with_name, &"a".repeat(60), true),
    ];
    for (rule, text, ok) in cases {
        assert_eq!(rule.check(text).is_ok(), ok, "{text}");
    }
    assert!(NameRule::of(NameFormat::Label).check("web-").is_err());
}
