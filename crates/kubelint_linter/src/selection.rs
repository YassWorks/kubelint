//! Rule selection: which rules run, and at which severity (`docs/user/configuration.md`).
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
    Select,
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
    pub list: SelectorList,
    /// Its position in that list, so configuration errors can point at it.
    pub index: usize,
    /// The selector as written.
    pub selector: String,
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

/// Whether the run fails: any diagnostic at or above `fail_on`.
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
mod tests;
