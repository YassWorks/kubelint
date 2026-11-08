use kubelint_diagnostics::Severity;

use super::*;
use crate::{ObjectContext, RuleMeta};

/// A rule with metadata only, for a registry that has rules disabled by default.
struct Fake(&'static RuleMeta);

impl Rule for Fake {
    fn meta(&self) -> &'static RuleMeta {
        self.0
    }

    fn check_object(&self, _: &mut ObjectContext<'_>) {}
}

macro_rules! fake {
    ($code:literal, $name:literal, $category:ident, $severity:ident, $on:literal) => {
        &Fake(&RuleMeta {
            code: $code,
            name: $name,
            category: Category::$category,
            default_severity: Severity::$severity,
            default_enabled: $on,
            summary: "",
            explanation: "",
        })
    };
}

static REGISTRY: &[&dyn Rule] = &[
    &InvalidYaml,
    fake!("KL0002", "dup", Syntax, Error, true),
    fake!("KL1001", "no-api", Correctness, Error, true),
    fake!("KL1002", "no-kind", Correctness, Error, true),
    fake!("KL1101", "schema", Correctness, Error, false),
    fake!("KL2001", "privileged", Security, Error, true),
    fake!("KL2009", "writable-root", Security, Warning, false),
    fake!("KL3001", "no-memory-limit", Reliability, Warning, true),
    fake!("KL3004", "no-probe", Reliability, Warning, false),
    fake!("KL3005", "one-replica", Reliability, Info, false),
];

fn strings(selectors: &[&str]) -> Vec<String> {
    selectors.iter().map(|&s| s.to_owned()).collect()
}

fn selection(select: Option<&[&str]>, ignore: &[&str]) -> RuleSelection {
    RuleSelection {
        select: select.map(strings),
        ignore: strings(ignore),
        severity: Vec::new(),
    }
}

/// The enabled codes, with the severity when it differs from the default.
fn resolve(selection: &RuleSelection) -> Result<Vec<String>, SelectionError> {
    Ok(selection
        .resolve_in(REGISTRY)?
        .iter()
        .map(|r| {
            let meta = r.rule.meta();
            if r.severity == meta.default_severity {
                meta.code.to_owned()
            } else {
                format!("{}={}", meta.code, r.severity.as_str())
            }
        })
        .collect())
}

fn codes(select: Option<&[&str]>, ignore: &[&str]) -> Vec<String> {
    resolve(&selection(select, ignore)).expect("resolves")
}

fn err(selection: &RuleSelection) -> (SelectorList, usize, SelectionErrorKind) {
    let error = resolve(selection).expect_err("fails");
    (error.list, error.index, error.kind)
}

#[test]
fn default_set_is_every_rule_enabled_by_default() {
    let expected = ["KL0001", "KL0002", "KL1001", "KL1002", "KL2001", "KL3001"];
    assert_eq!(codes(None, &[]), expected);
    assert_eq!(resolve(&RuleSelection::default()).unwrap(), expected);
}

#[test]
fn categories_and_prefixes_select_the_rules_enabled_by_default() {
    assert_eq!(codes(Some(&["security"]), &[]), ["KL0001", "KL2001"]);
    assert_eq!(codes(Some(&["KL2"]), &[]), ["KL0001", "KL2001"]);
    assert_eq!(codes(Some(&["KL10"]), &[]), ["KL0001", "KL1001", "KL1002"]);
    assert_eq!(codes(Some(&["KL100"]), &[]), ["KL0001", "KL1001", "KL1002"]);
    assert_eq!(codes(Some(&["reliability"]), &[]), ["KL0001", "KL3001"]);
}

#[test]
fn codes_and_names_select_rules_disabled_by_default() {
    assert_eq!(codes(Some(&["KL2009"]), &[]), ["KL0001", "KL2009"]);
    assert_eq!(codes(Some(&["writable-root"]), &[]), ["KL0001", "KL2009"]);
    // `security` does not bring in KL2009, which is disabled by default.
    assert_eq!(
        codes(Some(&["security", "no-probe", "KL3005"]), &[]),
        ["KL0001", "KL2001", "KL3004", "KL3005"]
    );
}

#[test]
fn a_prefix_whose_rules_are_all_disabled_selects_nothing_but_is_known() {
    assert_eq!(codes(Some(&["KL11"]), &[]), ["KL0001"]);
    assert_eq!(codes(None, &["KL110"]), codes(None, &[]));
}

#[test]
fn select_is_a_union_and_duplicates_are_harmless() {
    assert_eq!(
        codes(Some(&["KL1001", "correctness", "no-api"]), &[]),
        ["KL0001", "KL1001", "KL1002"]
    );
}

#[test]
fn empty_select_runs_only_invalid_yaml() {
    assert_eq!(codes(Some(&[]), &[]), ["KL0001"]);
}

#[test]
fn invalid_yaml_runs_whatever_is_selected_or_ignored() {
    assert_eq!(codes(Some(&["KL2001"]), &[]), ["KL0001", "KL2001"]);
    assert_eq!(
        codes(None, &["syntax"]),
        ["KL0001", "KL1001", "KL1002", "KL2001", "KL3001"]
    );
    assert_eq!(
        codes(None, &["KL0"]),
        ["KL0001", "KL1001", "KL1002", "KL2001", "KL3001"]
    );
}

#[test]
fn ignore_wins_over_select() {
    assert_eq!(
        codes(None, &["KL2001", "reliability"]),
        ["KL0001", "KL0002", "KL1001", "KL1002"]
    );
    assert_eq!(codes(Some(&["KL2001"]), &["security"]), ["KL0001"]);
    // Ignoring by code or name also removes rules disabled by default.
    assert_eq!(codes(Some(&["KL2009"]), &["writable-root"]), ["KL0001"]);
    assert_eq!(codes(Some(&["KL2009"]), &["KL2009"]), ["KL0001"]);
}

#[test]
fn ignoring_a_category_or_prefix_leaves_explicitly_selected_off_rules_alone() {
    // Broad selectors only cover rules enabled by default, in `ignore` too.
    assert_eq!(
        codes(Some(&["KL2009"]), &["security"]),
        ["KL0001", "KL2009"]
    );
}

#[test]
fn unknown_selectors_are_errors() {
    for bad in [
        "KL9999",
        "KL4",
        "KL",
        "kl2001",
        "Security",
        "privileged-container",
        "",
        "KL20011",
    ] {
        let error = (SelectorList::Select, 0, SelectionErrorKind::Unknown);
        assert_eq!(err(&selection(Some(&[bad]), &[])), error, "{bad:?}");
    }
    let error = (SelectorList::Ignore, 1, SelectionErrorKind::Unknown);
    assert_eq!(err(&selection(None, &["KL2001", "KL2010"])), error);
}

#[test]
fn invalid_yaml_cannot_be_ignored_or_overridden() {
    let error = (SelectorList::Ignore, 0, SelectionErrorKind::AlwaysOn);
    assert_eq!(err(&selection(None, &["KL0001"])), error);
    assert_eq!(err(&selection(None, &["invalid-yaml"])), error);
    let overridden = RuleSelection {
        severity: vec![("KL0001".to_owned(), Severity::Warning)],
        ..RuleSelection::default()
    };
    let error = (SelectorList::Severity, 0, SelectionErrorKind::AlwaysOn);
    assert_eq!(err(&overridden), error);
}

#[test]
fn severity_overrides_by_code_or_name_later_wins() {
    let selection = RuleSelection {
        select: Some(strings(&["KL2", "KL3", "no-probe"])),
        ignore: Vec::new(),
        severity: vec![
            ("KL3001".to_owned(), Severity::Error),
            ("privileged".to_owned(), Severity::Info),
            ("no-probe".to_owned(), Severity::Error),
            ("KL3004".to_owned(), Severity::Info),
            // An override for a rule that does not run is accepted and changes nothing.
            ("KL3005".to_owned(), Severity::Error),
        ],
    };
    assert_eq!(
        resolve(&selection).unwrap(),
        ["KL0001", "KL2001=info", "KL3001=error", "KL3004=info"]
    );
}

#[test]
fn severity_overrides_need_one_known_rule() {
    let by = |selector: &str| RuleSelection {
        severity: vec![
            ("KL1001".to_owned(), Severity::Warning),
            (selector.to_owned(), Severity::Warning),
        ],
        ..RuleSelection::default()
    };
    assert_eq!(
        err(&by("security")),
        (SelectorList::Severity, 1, SelectionErrorKind::NotOneRule)
    );
    assert_eq!(
        err(&by("KL2")),
        (SelectorList::Severity, 1, SelectionErrorKind::NotOneRule)
    );
    assert_eq!(
        err(&by("KL2999")),
        (SelectorList::Severity, 1, SelectionErrorKind::Unknown)
    );
}

#[test]
fn error_messages() {
    let message = |selection: &RuleSelection| resolve(selection).expect_err("fails").to_string();
    assert_eq!(
        message(&selection(Some(&["KL9"]), &[])),
        "unknown rule selector `KL9` in `select`: expected a rule code (`KL2001`), a code \
         prefix (`KL2`), a category (`security`) or a rule name"
    );
    assert_eq!(
        message(&selection(Some(&["Security"]), &[])),
        "unknown rule selector `Security` in `select`: did you mean `security`?"
    );
    assert_eq!(
        message(&selection(None, &["kl2001"])),
        "unknown rule selector `kl2001` in `ignore`: did you mean `KL2001`?"
    );
    assert_eq!(
        message(&selection(None, &["invalid-yaml"])),
        "`invalid-yaml` cannot be configured: KL0001 `invalid-yaml` always runs and is \
         always an error"
    );
    let overridden = RuleSelection {
        severity: vec![("security".to_owned(), Severity::Info)],
        ..RuleSelection::default()
    };
    assert_eq!(
        message(&overridden),
        "severity overrides take one rule code or name, not `security`"
    );
}

#[test]
fn real_registry_resolves_to_the_default_rules() {
    let resolved = RuleSelection::default().resolve().unwrap();
    let defaults = crate::Linter::with_default_rules();
    let codes =
        |rules: &[EnabledRule]| -> Vec<_> { rules.iter().map(|r| r.rule.meta().code).collect() };
    assert_eq!(codes(&resolved), codes(defaults.rules()));
}

#[test]
fn fails_at_or_above_the_threshold() {
    use kubelint_diagnostics::Label;
    use kubelint_source::{FileId, TextRange};
    let warning = Diagnostic::new(
        "KL3001",
        Severity::Warning,
        "x",
        Label::new(FileId(0), TextRange::empty(0.into())),
    );
    let found = [warning];
    assert!(fails(&found, Severity::Info));
    assert!(fails(&found, Severity::Warning));
    assert!(!fails(&found, Severity::Error));
    assert!(!fails(&[], Severity::Info));
}
