use super::*;

#[test]
fn table_lists_every_rule_in_code_order() {
    let table = list(RulesFormat::Text);
    let codes: Vec<_> = table
        .lines()
        .skip(1)
        .filter_map(|line| line.split_whitespace().next())
        .collect();
    let expected: Vec<_> = metas().map(|meta| meta.code).collect();
    assert_eq!(codes, expected);
    assert!(table.starts_with("CODE    NAME "), "{table}");
}

#[test]
fn json_is_an_array_of_rules() {
    let value: serde_json::Value =
        serde_json::from_str(&list(RulesFormat::Json)).expect("valid JSON");
    let rules = value.as_array().expect("an array");
    assert_eq!(rules.len(), rules::ALL.len());
    assert_eq!(rules[0]["code"], "KL0001");
    assert_eq!(rules[0]["default_enabled"], true);
}

#[test]
fn explain_by_code_or_name() {
    let by_code = explain("KL1002").expect("a rule");
    assert_eq!(Some(by_code.clone()), explain("missing-kind"));
    assert!(
        by_code.starts_with("# KL1002 `missing-kind`\n\n"),
        "{by_code}"
    );
    assert_eq!(explain("KL1"), None);
}
