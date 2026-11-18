use super::*;

fn codes(settings: &Settings) -> Vec<&'static str> {
    settings.rules.iter().map(|r| r.rule.meta().code).collect()
}

fn config(text: &str) -> Config {
    kubelint_config::parse(text).expect("valid config")
}

#[test]
fn no_config_and_no_flags_is_the_default_set() {
    let settings = resolve(
        &Config::default(),
        &Overrides::default(),
        KubernetesVersion::LATEST,
    )
    .expect("valid");
    let defaults: Vec<_> = kubelint_linter::rules::ALL
        .iter()
        .map(|rule| rule.meta())
        .filter(|meta| meta.default_enabled)
        .map(|meta| meta.code)
        .collect();
    assert_eq!(codes(&settings), defaults);
    assert_eq!(settings.fail_on, Severity::Warning);
}

#[test]
fn flags_replace_the_file_lists() {
    let config = config("[lint]\nselect = [\"KL1001\"]\nignore = [\"KL1002\"]\n");
    let from_file =
        resolve(&config, &Overrides::default(), KubernetesVersion::LATEST).expect("valid");
    assert_eq!(codes(&from_file), ["KL0001", "KL1001"]);
    let overrides = Overrides {
        select: Some(vec!["KL1001".to_owned(), "KL1002".to_owned()]),
        ignore: Some(Vec::new()),
        fail_on: Some(Severity::Error),
        kubernetes_version: None,
    };
    let from_flags = resolve(&config, &overrides, KubernetesVersion::LATEST).expect("valid");
    assert_eq!(codes(&from_flags), ["KL0001", "KL1001", "KL1002"]);
    assert_eq!(from_flags.fail_on, Severity::Error);
}

#[test]
fn severity_and_fail_on_come_from_the_file() {
    let config = config(include_str!(
        "../../tests/fixtures/settings/fail-on-and-severity.toml"
    ));
    let settings =
        resolve(&config, &Overrides::default(), KubernetesVersion::LATEST).expect("valid");
    assert_eq!(settings.fail_on, Severity::Error);
    let kl1002 = settings
        .rules
        .iter()
        .find(|r| r.rule.meta().code == "KL1002")
        .expect("KL1002 runs");
    assert_eq!(kl1002.severity, Severity::Info);
}

#[test]
fn errors_point_into_the_file_unless_a_flag_was_wrong() {
    let text = "[lint]\nselect = [\"KL1001\", \"secrity\"]\n";
    let (err, range) = resolve(
        &config(text),
        &Overrides::default(),
        KubernetesVersion::LATEST,
    )
    .expect_err("unknown");
    assert_eq!(err.selector, "secrity");
    let range = range.expect("in the file");
    let at = &text[usize::from(range.start())..usize::from(range.end())];
    assert_eq!(at, "\"secrity\"");

    let overrides = Overrides {
        ignore: Some(vec!["KL9".to_owned()]),
        ..Overrides::default()
    };
    let (err, range) =
        resolve(&Config::default(), &overrides, KubernetesVersion::LATEST).expect_err("unknown");
    assert_eq!((err.selector.as_str(), range), ("KL9", None));
}

#[test]
fn kubernetes_version_comes_from_the_flag_then_the_file() {
    let version = |config: &Config, overrides: &Overrides| {
        kubernetes_version(config, overrides).map(KubernetesVersion::as_str)
    };
    let latest = KubernetesVersion::LATEST.as_str();
    assert_eq!(
        version(&Config::default(), &Overrides::default()),
        Ok(latest)
    );
    let file = config("[lint]\nkubernetes-version = \"1.35.4\"\n");
    assert_eq!(version(&file, &Overrides::default()), Ok("1.35"));
    let flag = Overrides {
        kubernetes_version: Some("v1.36".to_owned()),
        ..Overrides::default()
    };
    assert_eq!(version(&file, &flag), Ok("1.36"));
    let unsupported = config("[lint]\nkubernetes-version = \"1.29\"\n");
    assert_eq!(version(&unsupported, &flag), Ok("1.36"));
}

#[test]
fn unsupported_kubernetes_versions_point_into_the_file_unless_a_flag_was_wrong() {
    let text = "[lint]\nkubernetes-version = \"1.20\"\n";
    let (message, range) =
        kubernetes_version(&config(text), &Overrides::default()).expect_err("unsupported");
    assert_eq!(
        message,
        "unsupported Kubernetes version `1.20`: kubelint supports 1.34, 1.35, 1.36 and 1.37"
    );
    let range = range.expect("in the file");
    assert_eq!(
        &text[usize::from(range.start())..usize::from(range.end())],
        "\"1.20\""
    );
    let flag = |value: &str| Overrides {
        kubernetes_version: Some(value.to_owned()),
        ..Overrides::default()
    };
    for (value, expected) in [
        ("1.38", "unsupported Kubernetes version `1.38`: "),
        (
            "1.37\nerror: x",
            "unsupported Kubernetes version `1.37\\nerror: x`: ",
        ),
        (
            "\u{1b}[31m1.37",
            "unsupported Kubernetes version `\\u{1b}[31m1.37`: ",
        ),
        ("", "the Kubernetes version is empty: "),
    ] {
        let (message, range) =
            kubernetes_version(&config(text), &flag(value)).expect_err("unsupported");
        assert!(message.starts_with(expected), "{message}");
        assert_eq!(range, None);
    }
}
