//! The lint engine: parse files, find objects, run rules, sort the findings.

use kubelint_diagnostics::{Diagnostic, Severity};
use kubelint_source::{FileId, SourceFile};
use kubelint_yaml::Parse;
use rayon::prelude::*;

use crate::context::{LintObject, ObjectContext, RunContext};
use crate::rules::syntax::{InvalidSuppression, InvalidYaml};
use crate::suppression::Suppressions;
use crate::{Rule, rules};

/// A rule switched on for a run, with the severity it reports at.
#[derive(Clone, Copy)]
pub struct EnabledRule {
    pub rule: &'static dyn Rule,
    /// The severity of its findings (its default, or a configured override).
    pub severity: Severity,
}

impl EnabledRule {
    /// Enables `rule` at its default severity.
    pub fn new(rule: &'static dyn Rule) -> Self {
        Self {
            rule,
            severity: rule.meta().default_severity,
        }
    }
}

impl std::fmt::Debug for EnabledRule {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EnabledRule")
            .field("code", &self.rule.meta().code)
            .field("severity", &self.severity)
            .finish()
    }
}

/// The result of [`Linter::lint`].
#[derive(Debug, Default)]
pub struct LintResult {
    /// Findings sorted by file, then byte offset, then rule code.
    pub diagnostics: Vec<Diagnostic>,
    pub objects_checked: usize,
}

/// Runs a fixed set of rules over files.
#[derive(Debug, Clone)]
pub struct Linter {
    rules: Vec<EnabledRule>,
}

impl Linter {
    pub fn new(rules: Vec<EnabledRule>) -> Self {
        Self { rules }
    }

    /// A linter running every built-in rule that is enabled by default, at its default
    /// severity.
    pub fn with_default_rules() -> Self {
        Self::new(
            rules::ALL
                .iter()
                .filter(|rule| rule.meta().default_enabled)
                .map(|&rule| EnabledRule::new(rule))
                .collect(),
        )
    }

    pub fn rules(&self) -> &[EnabledRule] {
        &self.rules
    }

    /// Lints `files`. A file's [`FileId`] is its index in the slice.
    ///
    /// Files are independent until the cross-object rules run, so parsing, per-object
    /// rules and suppression comments are handled for each file in parallel (`rayon`).
    /// Cross-object rules ([`Rule::check_all`]) then run once over every object. The
    /// findings are sorted by file, byte offset and rule code, so the result does not
    /// depend on thread scheduling.
    ///
    /// YAML syntax errors are always reported as KL0001, whatever rules are enabled;
    /// the other documents of a file with an error are still checked. Findings silenced
    /// by suppression comments (`# kubelint-disable-line KL1001`) or by an object's
    /// `kubelint/ignore` annotation are dropped; KL0003 reports the suppressions that
    /// cannot work.
    pub fn lint(&self, files: &[SourceFile]) -> LintResult {
        let parses: Vec<_> = files
            .par_iter()
            .map(|file| kubelint_yaml::parse(file.text()))
            .collect();
        let per_file: Vec<FileLint<'_>> = files
            .par_iter()
            .zip(&parses)
            .enumerate()
            .map(|(index, (file, parse))| {
                self.lint_file(
                    FileId(u32::try_from(index).unwrap_or(u32::MAX)),
                    file,
                    parse,
                )
            })
            .collect();

        let objects: Vec<LintObject<'_>> = per_file
            .iter()
            .flat_map(|lint| lint.objects.iter().copied())
            .collect();
        let mut cross_object = Vec::new();
        for enabled in &self.rules {
            let mut cx = RunContext::new(
                files,
                &objects,
                enabled.rule.meta(),
                enabled.severity,
                &mut cross_object,
            );
            enabled.rule.check_all(&mut cx);
        }
        cross_object.retain(|diagnostic| {
            let index = diagnostic.primary.file.0 as usize;
            match (files.get(index), per_file.get(index)) {
                (Some(file), Some(lint)) => !lint.suppressions.suppresses(file, diagnostic),
                _ => true,
            }
        });

        let mut diagnostics: Vec<Diagnostic> = per_file
            .into_iter()
            .flat_map(|lint| lint.diagnostics)
            .chain(cross_object)
            .collect();
        diagnostics.sort_by(|a, b| {
            (a.primary.file, a.primary.range.start(), a.code).cmp(&(
                b.primary.file,
                b.primary.range.start(),
                b.code,
            ))
        });
        LintResult {
            diagnostics,
            objects_checked: objects.len(),
        }
    }

    /// Everything that needs only one file: its objects, its syntax errors, its invalid
    /// suppression comments and the per-object findings, minus the suppressed ones.
    fn lint_file<'a>(&self, id: FileId, file: &'a SourceFile, parse: &'a Parse) -> FileLint<'a> {
        let objects: Vec<LintObject<'a>> = parse
            .documents
            .iter()
            .flat_map(kubelint_k8s::objects)
            .map(|object| LintObject { file: id, object })
            .collect();

        let mut diagnostics: Vec<Diagnostic> = parse
            .errors
            .iter()
            .map(|error| InvalidYaml::diagnostic(id, error))
            .collect();
        let invalid_suppression = InvalidSuppression.meta().code;
        if let Some(enabled) = self
            .rules
            .iter()
            .find(|enabled| enabled.rule.meta().code == invalid_suppression)
        {
            diagnostics.extend(InvalidSuppression::diagnostics(
                id,
                &parse.comments,
                &objects,
                enabled.severity,
            ));
        }
        for enabled in &self.rules {
            let meta = enabled.rule.meta();
            for lint_object in &objects {
                let mut cx = ObjectContext::new(
                    id,
                    file,
                    lint_object.object,
                    meta,
                    enabled.severity,
                    &mut diagnostics,
                );
                enabled.rule.check_object(&mut cx);
            }
        }

        let suppressions = Suppressions::new(file, &parse.comments, &objects);
        diagnostics.retain(|diagnostic| !suppressions.suppresses(file, diagnostic));
        FileLint {
            objects,
            diagnostics,
            suppressions,
        }
    }
}

/// The part of a run that [`Linter::lint_file`] computes for one file.
struct FileLint<'a> {
    objects: Vec<LintObject<'a>>,
    diagnostics: Vec<Diagnostic>,
    suppressions: Suppressions,
}
