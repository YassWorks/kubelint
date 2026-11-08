use kubelint_diagnostics::Severity;
use kubelint_linter::{Category, EnabledRule, Linter, ObjectContext, Rule, RuleMeta, RunContext};
use kubelint_source::SourceFile;

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
        SourceFile::new("a.yaml", include_str!("../resources/test/engine/a.yaml")),
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
        SourceFile::new(
            "bad.yaml",
            include_str!("../resources/test/engine/bad.yaml"),
        ),
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

/// Many files mixing objects, syntax errors, duplicate keys and suppression comments, so
/// every step of the engine has findings to order. `{i}` in a template is the file's
/// index, which keeps object names unique.
fn many_files() -> Vec<SourceFile> {
    let templates = [
        include_str!("../resources/test/engine/many/pod.yaml"),
        include_str!("../resources/test/engine/many/duplicate_key.yaml"),
        include_str!("../resources/test/engine/many/syntax_error.yaml"),
        include_str!("../resources/test/engine/many/suppressed.yaml"),
    ];
    (0..64)
        .map(|i| {
            let text = templates[i % templates.len()].replace("{i}", &i.to_string());
            SourceFile::new(format!("f{i}.yaml"), text)
        })
        .collect()
}

#[test]
fn parallel_output_matches_a_single_thread_and_is_stable() {
    let mut rules: Vec<EnabledRule> = kubelint_linter::rules::ALL
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
    let expected = kubelint_linter::rules::ALL
        .iter()
        .filter(|r| r.meta().default_enabled)
        .count();
    assert_eq!(linter.rules().len(), expected);
}
