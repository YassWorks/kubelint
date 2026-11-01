//! The [`Rule`] trait and the metadata every rule carries.

use kubelint_diagnostics::Severity;

use crate::{ObjectContext, RunContext};

/// One lint rule.
///
/// Rules are unit structs. A rule looks at one object at a time in
/// [`check_object`](Rule::check_object), or at every object of the run at once in
/// [`check_all`](Rule::check_all) (for cross-object rules such as duplicate detection).
/// It reports findings through the context and never fails: a rule that cannot decide
/// (a field is missing or has the wrong shape) reports nothing.
///
/// Rules are pure, with no I/O and no global state, so the engine can run them in
/// parallel (`Sync`).
pub trait Rule: Sync {
    /// The rule's code, name, category and documentation.
    fn meta(&self) -> &'static RuleMeta;

    /// Checks one Kubernetes object. Called once per object.
    fn check_object(&self, cx: &mut ObjectContext<'_>) {
        let _ = cx;
    }

    /// Checks all objects of the run together. Called once per run.
    fn check_all(&self, cx: &mut RunContext<'_>) {
        let _ = cx;
    }
}

/// What a rule is and how it behaves by default. Codes and names are permanent once
/// released (ADR 0004).
#[derive(Debug)]
pub struct RuleMeta {
    /// `KL` and four digits, allocated in `docs/rules/README.md`. The first digit is the
    /// category.
    pub code: &'static str,
    /// Kebab-case description of the problem, such as `privileged-container`.
    pub name: &'static str,
    /// The rule's category.
    pub category: Category,
    /// Severity unless configuration overrides it.
    pub default_severity: Severity,
    /// Whether the rule runs without being selected explicitly.
    pub default_enabled: bool,
    /// One line, shown by `kubelint rules`.
    pub summary: &'static str,
    /// Markdown: why it matters, examples, how to fix it.
    pub explanation: &'static str,
}

/// The group a rule belongs to. Users can select a whole category by name (`security`)
/// or by code prefix (`KL2`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Category {
    /// `KL0xxx`: the file cannot be read as YAML.
    Syntax,
    /// `KL1xxx`: Kubernetes would reject the object or misread it.
    Correctness,
    /// `KL2xxx`: the object weakens the cluster's security.
    Security,
    /// `KL3xxx`: the workload is likely to misbehave in production.
    Reliability,
}

impl Category {
    /// Every category, in code order.
    pub const ALL: [Self; 4] = [
        Self::Syntax,
        Self::Correctness,
        Self::Security,
        Self::Reliability,
    ];

    /// The lowercase name used in configuration and output.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Syntax => "syntax",
            Self::Correctness => "correctness",
            Self::Security => "security",
            Self::Reliability => "reliability",
        }
    }

    /// The code prefix of the category's rules, such as `KL2`.
    ///
    /// ```
    /// use kubelint_linter::Category;
    ///
    /// assert_eq!(Category::Security.code_prefix(), "KL2");
    /// ```
    pub fn code_prefix(self) -> &'static str {
        match self {
            Self::Syntax => "KL0",
            Self::Correctness => "KL1",
            Self::Security => "KL2",
            Self::Reliability => "KL3",
        }
    }
}
