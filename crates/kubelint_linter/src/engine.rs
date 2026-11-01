//! The lint engine: parse files, find objects, run rules, sort the findings.

use kubelint_diagnostics::{Diagnostic, Severity};
use kubelint_source::{FileId, SourceFile};

use crate::context::{LintObject, ObjectContext, RunContext};
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
    /// Files are linted one after another; parallel linting is roadmap task M2-06.
    /// Syntax errors are not reported yet (M1-11): objects in documents before the first
    /// error are still checked.
    pub fn lint(&self, files: &[SourceFile]) -> LintResult {
        let parses: Vec<_> = files
            .iter()
            .map(|file| kubelint_yaml::parse(file.text()))
            .collect();
        let objects: Vec<LintObject<'_>> = parses
            .iter()
            .enumerate()
            .flat_map(|(index, parse)| {
                let file = FileId(u32::try_from(index).unwrap_or(u32::MAX));
                parse
                    .documents
                    .iter()
                    .flat_map(kubelint_k8s::objects)
                    .map(move |object| LintObject { file, object })
            })
            .collect();

        let mut diagnostics = Vec::new();
        for enabled in &self.rules {
            let meta = enabled.rule.meta();
            for lint_object in &objects {
                let Some(file) = files.get(lint_object.file.0 as usize) else {
                    continue;
                };
                let mut cx = ObjectContext::new(
                    lint_object.file,
                    file,
                    lint_object.object,
                    meta,
                    enabled.severity,
                    &mut diagnostics,
                );
                enabled.rule.check_object(&mut cx);
            }
            let mut cx = RunContext::new(files, &objects, meta, enabled.severity, &mut diagnostics);
            enabled.rule.check_all(&mut cx);
        }

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
    fn default_rules_are_the_default_enabled_built_ins() {
        let linter = Linter::with_default_rules();
        let expected = crate::rules::ALL
            .iter()
            .filter(|r| r.meta().default_enabled)
            .count();
        assert_eq!(linter.rules().len(), expected);
    }
}
