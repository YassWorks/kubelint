use std::collections::HashSet;

use super::ALL;
use crate::testing::check_fixture;

#[test]
fn fixtures() {
    for &rule in ALL {
        insta::assert_snapshot!(rule.meta().code, check_fixture(rule));
    }
}

#[test]
fn codes_and_names_are_unique_and_well_formed() {
    let mut codes = HashSet::new();
    let mut names = HashSet::new();
    for rule in ALL {
        let meta = rule.meta();
        assert!(codes.insert(meta.code), "duplicate code {}", meta.code);
        assert!(names.insert(meta.name), "duplicate name {}", meta.name);
        assert!(
            meta.code.len() == 6
                && meta.code.starts_with(meta.category.code_prefix())
                && meta.code[2..].bytes().all(|b| b.is_ascii_digit()),
            "{} must be KL + 4 digits in its category's range",
            meta.code
        );
        assert!(
            !meta.name.is_empty()
                && meta
                    .name
                    .bytes()
                    .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-'),
            "{} must be kebab-case",
            meta.name
        );
    }
}

#[test]
fn registry_is_in_code_order() {
    let codes: Vec<_> = ALL.iter().map(|r| r.meta().code).collect();
    assert!(codes.is_sorted(), "{codes:?}");
}
