use super::*;

fn codes(settings: &Settings) -> Vec<&'static str> {
    settings.rules.iter().map(|r| r.rule.meta().code).collect()
}

fn config(text: &str) -> Config {
    kubelint_config::parse(text).expect("valid config")
}

#[test]
fn no_config_and_no_flags_is_the_default_set() {
    let settings = resolve(&Config::default(), &Overrides::default()).expect("valid");
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
    let from_file = resolve(&config, &Overrides::default()).expect("valid");
    assert_eq!(codes(&from_file), ["KL0001", "KL1001"]);
    let overrides = Overrides {
        select: Some(vec!["KL1001".to_owned(), "KL1002".to_owned()]),
        ignore: Some(Vec::new()),
        fail_on: Some(Severity::Error),
    };
    let from_flags = resolve(&config, &overrides).expect("valid");
    assert_eq!(codes(&from_flags), ["KL0001", "KL1001", "KL1002"]);
    assert_eq!(from_flags.fail_on, Severity::Error);
}

#[test]
fn severity_and_fail_on_come_from_the_file() {
    let config = config(include_str!(
        "../../tests/fixtures/settings/fail-on-and-severity.toml"
    ));
    let settings = resolve(&config, &Overrides::default()).expect("valid");
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
    let (err, range) = resolve(&config(text), &Overrides::default()).expect_err("unknown");
    assert_eq!(err.selector, "secrity");
    let range = range.expect("in the file");
    let at = &text[usize::from(range.start())..usize::from(range.end())];
    assert_eq!(at, "\"secrity\"");

    let overrides = Overrides {
        ignore: Some(vec!["KL9".to_owned()]),
        ..Overrides::default()
    };
    let (err, range) = resolve(&Config::default(), &overrides).expect_err("unknown");
    assert_eq!((err.selector.as_str(), range), ("KL9", None));
}
