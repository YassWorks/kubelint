//! Rule selection: which rules run, and at which severity (ADR 0005, ADR 0009).
//!
//! Configuration and the command line name rules with *selectors*, plain strings that
//! this module resolves against the rule registry:
//!
//! | Selector | Example | Matches |
//! | --- | --- | --- |
//! | Category | `security` | the category's rules that are enabled by default |
//! | Code prefix | `KL2`, `KL20` | the rules enabled by default whose code starts with it |
//! | Code | `KL2009` | that rule, even if it is disabled by default |
//! | Name | `writable-root-filesystem` | that rule, even if it is disabled by default |
//!
//! `ignore` is applied after `select` and wins. KL0001 always runs. A selector that
//! matches no rule is an error.

use std::fmt;

use kubelint_diagnostics::{Diagnostic, Severity};

use crate::rules::syntax::InvalidYaml;
use crate::suggest::rule_suggestion;
use crate::{Category, EnabledRule, Rule, rules};

/// Which rules to run, as written by the user.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RuleSelection {
    /// The rules to start from. `None` means every rule that is enabled by default.
    pub select: Option<Vec<String>>,
    /// Rules to remove from the selection. Wins over `select`.
    pub ignore: Vec<String>,
    /// Severity overrides, by rule code or name. Later entries win.
    pub severity: Vec<(String, Severity)>,
}

/// Where a selector that could not be resolved was written.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SelectorList {
    /// In `select`.
    Select,
    /// In `ignore`.
    Ignore,
    /// A key of `severity`.
    Severity,
}

impl SelectorList {
    /// The configuration key: `select`, `ignore` or `severity`.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Select => "select",
            Self::Ignore => "ignore",
            Self::Severity => "severity",
        }
    }
}

/// A selector that could not be resolved.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SelectionError {
    /// The list the selector is in.
    pub list: SelectorList,
    /// Its position in that list, so configuration errors can point at it.
    pub index: usize,
    /// The selector as written.
    pub selector: String,
    /// What is wrong with it.
    pub kind: SelectionErrorKind,
    /// For an unknown selector, the closest rule code, rule name or category, if one is
    /// close enough to be a likely typo.
    pub suggestion: Option<&'static str>,
}

/// What is wrong with a selector.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SelectionErrorKind {
    /// Not a category, code, code prefix or name of any rule.
    Unknown,
    /// KL0001 `invalid-yaml` was named in `ignore` or `severity`; it is always on and
    /// always an error.
    AlwaysOn,
    /// A severity override names a category or code prefix instead of one rule.
    NotOneRule,
}

impl fmt::Display for SelectionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let selector = &self.selector;
        match self.kind {
            SelectionErrorKind::Unknown => match self.suggestion {
                Some(suggestion) => write!(
                    f,
                    "unknown rule selector `{selector}` in `{}`: did you mean `{suggestion}`?",
                    self.list.as_str()
                ),
                None => write!(
                    f,
                    "unknown rule selector `{selector}` in `{}`: expected a rule code \
                     (`KL2001`), a code prefix (`KL2`), a category (`security`) or a rule \
                     name",
                    self.list.as_str()
                ),
            },
            SelectionErrorKind::AlwaysOn => write!(
                f,
                "`{selector}` cannot be configured: KL0001 `invalid-yaml` always runs and \
                 is always an error"
            ),
            SelectionErrorKind::NotOneRule => write!(
                f,
                "severity overrides take one rule code or name, not `{selector}`"
            ),
        }
    }
}

impl std::error::Error for SelectionError {}

impl RuleSelection {
    /// The rules to run, in registry (code) order. KL0001 is always included.
    ///
    /// ```
    /// use kubelint_linter::RuleSelection;
    ///
    /// let selection = RuleSelection {
    ///     select: Some(vec!["KL1001".to_owned(), "missing-kind".to_owned()]),
    ///     ignore: vec!["missing-kind".to_owned()],
    ///     ..RuleSelection::default()
    /// };
    /// let codes: Vec<_> = selection
    ///     .resolve()
    ///     .unwrap()
    ///     .iter()
    ///     .map(|r| r.rule.meta().code)
    ///     .collect();
    /// assert_eq!(codes, ["KL0001", "KL1001"]);
    /// ```
    pub fn resolve(&self) -> Result<Vec<EnabledRule>, SelectionError> {
        self.resolve_in(rules::ALL)
    }

    fn resolve_in(
        &self,
        registry: &[&'static dyn Rule],
    ) -> Result<Vec<EnabledRule>, SelectionError> {
        let always_on = InvalidYaml.meta().code;
        let mut enabled: Vec<bool> = match &self.select {
            None => registry.iter().map(|r| r.meta().default_enabled).collect(),
            Some(select) => {
                let mut enabled = vec![false; registry.len()];
                for (index, selector) in select.iter().enumerate() {
                    let matched = matches(registry, selector, SelectorList::Select, index)?;
                    for (on, hit) in enabled.iter_mut().zip(matched) {
                        *on |= hit;
                    }
                }
                enabled
            }
        };
        for (index, selector) in self.ignore.iter().enumerate() {
            let error = |kind| error(SelectorList::Ignore, index, selector, kind);
            if is_exact(registry, selector, always_on) {
                return Err(error(SelectionErrorKind::AlwaysOn));
            }
            let matched = matches(registry, selector, SelectorList::Ignore, index)?;
            for (on, hit) in enabled.iter_mut().zip(matched) {
                *on &= !hit;
            }
        }
        let mut severities: Vec<Severity> =
            registry.iter().map(|r| r.meta().default_severity).collect();
        for (index, (selector, severity)) in self.severity.iter().enumerate() {
            let error = |kind| error(SelectorList::Severity, index, selector, kind);
            if is_exact(registry, selector, always_on) {
                return Err(error(SelectionErrorKind::AlwaysOn));
            }
            let Some(i) = registry.iter().position(|r| is_rule(*r, selector)) else {
                // Unknown, or a valid selector that is not one rule (a category, a prefix).
                matches(registry, selector, SelectorList::Severity, index)?;
                return Err(error(SelectionErrorKind::NotOneRule));
            };
            severities[i] = *severity;
        }
        Ok(registry
            .iter()
            .zip(enabled.iter().zip(severities))
            .filter(|(rule, (on, _))| **on || rule.meta().code == always_on)
            .map(|(&rule, (_, severity))| EnabledRule { rule, severity })
            .collect())
    }
}

/// Whether the run fails: any diagnostic at or above `fail_on` (ADR 0003).
///
/// ```
/// use kubelint_diagnostics::Severity;
///
/// assert!(!kubelint_linter::fails(&[], Severity::Info));
/// ```
pub fn fails(diagnostics: &[Diagnostic], fail_on: Severity) -> bool {
    diagnostics.iter().any(|d| d.severity >= fail_on)
}

fn error(
    list: SelectorList,
    index: usize,
    selector: &str,
    kind: SelectionErrorKind,
) -> SelectionError {
    SelectionError {
        list,
        index,
        selector: selector.to_owned(),
        kind,
        suggestion: None,
    }
}

/// Whether `selector` names exactly the rule with code `code` (by code or name).
fn is_exact(registry: &[&'static dyn Rule], selector: &str, code: &str) -> bool {
    registry
        .iter()
        .any(|rule| rule.meta().code == code && is_rule(*rule, selector))
}

/// Whether `selector` is this rule's code or name.
fn is_rule(rule: &dyn Rule, selector: &str) -> bool {
    let meta = rule.meta();
    meta.code == selector || meta.name == selector
}

/// For each rule of `registry`, whether `selector` matches it. A selector that matches
/// no rule is an error, so typos never pass silently.
fn matches(
    registry: &[&'static dyn Rule],
    selector: &str,
    list: SelectorList,
    index: usize,
) -> Result<Vec<bool>, SelectionError> {
    let category = Category::ALL
        .into_iter()
        .find(|category| category.as_str() == selector);
    let is_prefix = selector.len() < 6
        && selector.starts_with("KL")
        && selector.len() > 2
        && selector[2..].bytes().all(|b| b.is_ascii_digit());
    let matched: Vec<bool> = registry
        .iter()
        .map(|rule| {
            let meta = rule.meta();
            if is_rule(*rule, selector) {
                true
            } else if let Some(category) = category {
                meta.category == category && meta.default_enabled
            } else {
                is_prefix && meta.code.starts_with(selector) && meta.default_enabled
            }
        })
        .collect();
    // A category or prefix whose rules are all disabled by default is still known.
    let known = category.is_some()
        || matched.iter().any(|&hit| hit)
        || (is_prefix && registry.iter().any(|r| r.meta().code.starts_with(selector)));
    if known {
        Ok(matched)
    } else {
        Err(SelectionError {
            suggestion: rule_suggestion(registry, selector, true),
            ..error(list, index, selector, SelectionErrorKind::Unknown)
        })
    }
}

#[cfg(test)]
mod tests {
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
        let message =
            |selection: &RuleSelection| resolve(selection).expect_err("fails").to_string();
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
        let codes = |rules: &[EnabledRule]| -> Vec<_> {
            rules.iter().map(|r| r.rule.meta().code).collect()
        };
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
}
