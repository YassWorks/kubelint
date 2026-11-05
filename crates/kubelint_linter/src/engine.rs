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
    /// The rule.
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
    /// How many Kubernetes objects were checked.
    pub objects_checked: usize,
}

/// Runs a fixed set of rules over files.
#[derive(Debug, Clone)]
pub struct Linter {
    rules: Vec<EnabledRule>,
}

impl Linter {
    /// A linter running exactly these rules.
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

    /// The rules this linter runs.
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
    /// by suppression comments (`# kubelint-disable-line KL1001`) are dropped; KL0003
    /// reports the suppression comments that cannot work.
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

        let suppressions = Suppressions::new(file, &parse.comments);
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

#[cfg(test)]
mod tests {
    use kubelint_diagnostics::Severity;
    use kubelint_source::SourceFile;

    use super::{EnabledRule, Linter};
    use crate::{Category, ObjectContext, Rule, RuleMeta, RunContext};

    /// Reports every object at its `kind` value.
    struct KindReporter;

    static KIND_REPORTER: RuleMeta = RuleMeta {
        code: "KL9001",
        name: "kind-reporter",
        category: Category::Correctness,
        default_severity: Severity::Warning,
        default_enabled: true,
        summary: "test rule",
        explanation: "test rule",
    };

    impl Rule for KindReporter {
        fn meta(&self) -> &'static RuleMeta {
            &KIND_REPORTER
        }

        fn check_object(&self, cx: &mut ObjectContext<'_>) {
            let object = cx.object();
            let Some(kind) = object.kind() else { return };
            let message = format!("kind {}", kind.as_str().unwrap_or("?"));
            cx.report(cx.diagnostic(kind.range, message));
        }
    }

    /// Reports once per run, at the start of the last object.
    struct CountReporter;

    static COUNT_REPORTER: RuleMeta = RuleMeta {
        code: "KL9000",
        name: "count-reporter",
        category: Category::Syntax,
        default_severity: Severity::Info,
        default_enabled: false,
        summary: "test rule",
        explanation: "test rule",
    };

    impl Rule for CountReporter {
        fn meta(&self) -> &'static RuleMeta {
            &COUNT_REPORTER
        }

        fn check_all(&self, cx: &mut RunContext<'_>) {
            let Some(last) = cx.objects().last() else {
                return;
            };
            assert!(cx.file(last.file).is_some());
            let message = format!("{} objects", cx.objects().len());
            cx.report(cx.diagnostic(last.file, last.object.range(), message));
        }
    }

    fn files() -> Vec<SourceFile> {
        vec![
            SourceFile::new(
                "a.yaml",
                "# é\nkind: Pod\n---\nname: not-an-object\n---\napiVersion: v1\nkind: List\nitems:\n- kind: Service\n- kind: Secret\n",
            ),
            SourceFile::new("b.yaml", "kind: ConfigMap\n"),
        ]
    }

    /// `file:start code severity message` per diagnostic.
    fn summary(linter: &Linter, files: &[SourceFile]) -> Vec<String> {
        linter
            .lint(files)
            .diagnostics
            .iter()
            .map(|d| {
                format!(
                    "{}:{} {} {} {}",
                    d.primary.file.0,
                    u32::from(d.primary.range.start()),
                    d.code,
                    d.severity.as_str(),
                    d.message
                )
            })
            .collect()
    }

    #[test]
    fn runs_object_and_run_rules_and_sorts_findings() {
        let linter = Linter::new(vec![
            EnabledRule::new(&KindReporter),
            EnabledRule::new(&CountReporter),
        ]);
        let files = files();
        assert_eq!(linter.lint(&files).objects_checked, 4);
        assert_eq!(
            summary(&linter, &files),
            [
                "0:11 KL9001 warning kind Pod",
                "0:84 KL9001 warning kind Service",
                "0:100 KL9001 warning kind Secret",
                "1:0 KL9000 info 4 objects",
                "1:6 KL9001 warning kind ConfigMap",
            ]
        );
    }

    #[test]
    fn severity_override_is_applied() {
        let linter = Linter::new(vec![EnabledRule {
            rule: &KindReporter,
            severity: Severity::Error,
        }]);
        let files = [SourceFile::new("c.yaml", "kind: Pod\n")];
        assert_eq!(summary(&linter, &files), ["0:6 KL9001 error kind Pod"]);
    }

    #[test]
    fn no_files_and_no_rules() {
        let result = Linter::new(Vec::new()).lint(&files());
        assert!(result.diagnostics.is_empty());
        assert_eq!(result.objects_checked, 4);
        let result = Linter::new(vec![EnabledRule::new(&CountReporter)]).lint(&[]);
        assert!(result.diagnostics.is_empty());
    }

    #[test]
    fn syntax_errors_are_reported_without_any_rule_enabled() {
        let files = [
            SourceFile::new("ok.yaml", "kind: Pod\n"),
            SourceFile::new("bad.yaml", "a: [\n---\nkind: Service\n---\nb: ]\n"),
        ];
        let linter = Linter::new(Vec::new());
        assert_eq!(linter.lint(&files).objects_checked, 2);
        let summary = summary(&linter, &files);
        assert_eq!(summary.len(), 2, "{summary:?}");
        assert!(
            summary
                .iter()
                .all(|line| line.starts_with("1:") && line.contains(" KL0001 error invalid YAML: ")),
            "{summary:?}"
        );
    }

    /// Many files mixing objects, syntax errors, duplicate keys and suppression
    /// comments, so every step of the engine has findings to order.
    fn many_files() -> Vec<SourceFile> {
        (0..64)
            .map(|i| {
                let text = match i % 4 {
                    0 => format!("apiVersion: v1\nkind: Pod\nmetadata:\n  name: p{i}\n"),
                    1 => format!("kind: Service\nkind: Service\n---\nmetadata: {{name: s{i}}}\n"),
                    2 => "a: [\n---\napiVersion: v1\n# kubelint-disable-next-line KL9999\nkind: Secret\n".to_owned(),
                    _ => "# kubelint-disable-file KL1001\nkind: ConfigMap\n---\napiVersion: v1\n".to_owned(),
                };
                SourceFile::new(format!("f{i}.yaml"), text)
            })
            .collect()
    }

    #[test]
    fn parallel_output_matches_a_single_thread_and_is_stable() {
        let mut rules: Vec<EnabledRule> = crate::rules::ALL
            .iter()
            .map(|&rule| EnabledRule::new(rule))
            .collect();
        rules.push(EnabledRule::new(&KindReporter));
        rules.push(EnabledRule::new(&CountReporter));
        let linter = Linter::new(rules);
        let files = many_files();

        let single = rayon::ThreadPoolBuilder::new()
            .num_threads(1)
            .build()
            .expect("a one-thread pool")
            .install(|| summary(&linter, &files));
        assert!(single.len() > files.len(), "{single:?}");
        for _ in 0..20 {
            let parallel = rayon::ThreadPoolBuilder::new()
                .num_threads(8)
                .build()
                .expect("an eight-thread pool")
                .install(|| summary(&linter, &files));
            assert_eq!(parallel, single);
        }
    }

    #[test]
    fn default_rules_are_the_default_enabled_built_ins() {
        let linter = Linter::with_default_rules();
        let expected = crate::rules::ALL
            .iter()
            .filter(|r| r.meta().default_enabled)
            .count();
        assert_eq!(linter.rules().len(), expected);
    }
}
