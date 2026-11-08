use super::{did_you_mean, edit_distance, suggest_rule};

#[test]
fn distances() {
    assert_eq!(edit_distance("", ""), 0);
    assert_eq!(edit_distance("abc", ""), 3);
    assert_eq!(edit_distance("", "ab"), 2);
    assert_eq!(edit_distance("kitten", "sitting"), 3);
    assert_eq!(edit_distance("flaw", "lawn"), 2);
    assert_eq!(edit_distance("é", "e"), 1);
}

#[test]
fn suggestions() {
    let candidates = ["KL1001", "KL1002", "missing-kind", "security"];
    assert_eq!(did_you_mean("kl1001", candidates), Some("KL1001"));
    assert_eq!(did_you_mean("KL1003", candidates), Some("KL1001"));
    assert_eq!(did_you_mean("Security", candidates), Some("security"));
    assert_eq!(did_you_mean("securty", candidates), Some("security"));
    assert_eq!(
        did_you_mean("missingkind", candidates),
        Some("missing-kind")
    );
    // Too far from everything, or too short to guess.
    assert_eq!(did_you_mean("reliability", candidates), None);
    assert_eq!(did_you_mean("KL", candidates), None);
    assert_eq!(did_you_mean("", candidates), None);
    assert_eq!(did_you_mean("x", candidates), None);
}

#[test]
fn suggests_built_in_rules_only() {
    assert_eq!(suggest_rule("KL0O01"), Some("KL0001"));
    assert_eq!(suggest_rule("invalid_yaml"), Some("invalid-yaml"));
    assert_eq!(suggest_rule("security"), None);
}
