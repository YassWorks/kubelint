use kubelint_config::{Config, Level, parse};
use kubelint_source::TextRange;

fn at(text: &str, range: TextRange) -> &str {
    &text[usize::from(range.start())..usize::from(range.end())]
}

/// The error message and the text it points at.
fn error(text: &str) -> (String, &str) {
    let error = parse(text).expect_err("invalid");
    let range = error.range.expect("has a position");
    (error.message, at(text, range))
}

#[test]
fn empty_file_is_the_default() {
    assert_eq!(parse(""), Ok(Config::default()));
    assert_eq!(parse("# nothing\n[lint]\n[files]\n"), Ok(Config::default()));
}

#[test]
fn every_key_with_its_position() {
    let text = include_str!("fixtures/every_key.toml");
    let config = parse(text).expect("valid");
    let select = config.lint.select.expect("select is set");
    let select: Vec<_> = select
        .iter()
        .map(|s| (s.value.as_str(), at(text, s.range)))
        .collect();
    assert_eq!(
        select,
        [("correctness", "\"correctness\""), ("KL3001", "\"KL3001\"")]
    );
    assert_eq!(config.lint.ignore[0].value, "KL2004");
    let fail_on = config.lint.fail_on.expect("fail-on is set");
    assert_eq!(
        (fail_on.value, at(text, fail_on.range)),
        (Level::Info, "\"info\"")
    );
    let severity: Vec<_> = config
        .lint
        .severity
        .iter()
        .map(|(key, level)| (at(text, key.range), *level))
        .collect();
    assert_eq!(
        severity,
        [("KL3003", Level::Error), ("missing-kind", Level::Warning)]
    );
    assert_eq!(at(text, config.files.exclude[0].range), "\"vendor/**\"");
}

#[test]
fn empty_select_is_kept_apart_from_no_select() {
    let config = parse("[lint]\nselect = []\n").expect("valid");
    assert_eq!(config.lint.select, Some(Vec::new()));
}

#[test]
fn unknown_keys_are_errors() {
    let (message, at) = error("[lint]\nselect = []\nfail_on = \"error\"\n");
    assert!(
        message.starts_with("unknown field `fail_on`, expected one of"),
        "{message}"
    );
    assert_eq!(at, "fail_on");
    let (message, at) = error("[lnt]\n");
    assert!(message.starts_with("unknown field `lnt`"), "{message}");
    assert_eq!(at, "lnt");
    let (message, _) = error("[files]\ninclude = []\n");
    assert!(message.starts_with("unknown field `include`"), "{message}");
}

#[test]
fn wrong_values_are_errors() {
    let (message, at) = error("[lint]\nfail-on = \"warn\"\n");
    assert!(message.starts_with("unknown variant `warn`"), "{message}");
    assert_eq!(at, "\"warn\"");
    let (message, at) = error("[lint]\nselect = \"security\"\n");
    assert!(message.contains("expected a sequence"), "{message}");
    assert_eq!(at, "\"security\"");
    let (message, at) = error("[lint.severity]\nKL3003 = \"fatal\"\n");
    assert!(message.starts_with("unknown variant `fatal`"), "{message}");
    assert_eq!(at, "\"fatal\"");
}

#[test]
fn invalid_toml_is_an_error() {
    let (message, _) = error("[lint\n");
    assert!(!message.is_empty());
}
