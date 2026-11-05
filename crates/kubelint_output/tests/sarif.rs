//! Tests of the SARIF renderer: an exact snapshot, and structural checks of the parts of
//! SARIF 2.1.0 that GitHub code scanning requires.

use kubelint_diagnostics::{Diagnostic, Label, Severity};
use kubelint_output::{SarifRenderer, SarifRule};
use kubelint_source::{FileId, SourceFile, TextRange};
use serde_json::Value;

const RULES: [SarifRule<'static>; 2] = [
    SarifRule {
        code: "KL1004",
        name: "duplicate-object",
        summary: "Two objects have the same kind, namespace and name.",
        explanation: "## Why\n\nThe second one silently replaces the first.\n",
        category: "correctness",
        default_severity: Severity::Error,
    },
    SarifRule {
        code: "KL3005",
        name: "single-replica",
        summary: "The workload runs a single replica.",
        explanation: "One pod means downtime on every restart.\n",
        category: "reliability",
        default_severity: Severity::Info,
    },
];

fn range(start: u32, end: u32) -> TextRange {
    TextRange::new(start.into(), end.into())
}

fn files() -> Vec<SourceFile> {
    vec![
        SourceFile::new("deploy\\a b.yaml", "kind: Pod\nname: é😀x\n"),
        SourceFile::new("/abs/b.yaml", "kind: Pod\n"),
    ]
}

fn diagnostics() -> Vec<Diagnostic> {
    vec![
        // `x` after `é😀`: 1 + 2 UTF-16 code units, 2 + 4 UTF-8 bytes.
        Diagnostic::new(
            "KL1004",
            Severity::Warning,
            "duplicate object",
            Label::new(FileId(0), range(22, 23)).with_message("second here"),
        )
        .with_help("rename one of them")
        .with_secondary(Label::new(FileId(1), range(0, 9)).with_message("first here")),
        Diagnostic::new(
            "KL3005",
            Severity::Info,
            "one replica",
            Label::new(FileId(1), range(6, 9)),
        ),
        Diagnostic::new(
            "KL0001",
            Severity::Error,
            "a code without a rule entry",
            Label::new(FileId(0), range(0, 0)),
        ),
    ]
}

fn render() -> String {
    SarifRenderer::new("1.2.3", &RULES).render(&files(), &diagnostics())
}

#[test]
fn snapshot() {
    insta::assert_snapshot!(render());
}

#[test]
fn has_the_fields_github_code_scanning_requires() {
    let log: Value = serde_json::from_str(&render()).expect("valid JSON");
    assert_eq!(log["version"], "2.1.0");
    assert!(
        log["$schema"]
            .as_str()
            .is_some_and(|s| s.contains("sarif-2.1.0"))
    );
    let runs = log["runs"].as_array().expect("runs");
    assert_eq!(runs.len(), 1);
    let run = &runs[0];
    let driver = &run["tool"]["driver"];
    assert_eq!(driver["name"], "kubelint");
    assert_eq!(driver["version"], "1.2.3");
    let rules = driver["rules"].as_array().expect("rules");
    assert_eq!(rules.len(), RULES.len());
    for rule in rules {
        assert!(rule["id"].is_string());
        assert!(rule["shortDescription"]["text"].is_string());
        assert!(rule["help"]["markdown"].is_string());
    }

    let results = run["results"].as_array().expect("results");
    assert_eq!(results.len(), 3);
    for result in results {
        let id = result["ruleId"].as_str().expect("ruleId");
        if let Some(index) = result["ruleIndex"].as_u64() {
            assert_eq!(rules[usize::try_from(index).expect("small")]["id"], id);
        }
        assert!(["error", "warning", "note"].contains(&result["level"].as_str().expect("level")));
        assert!(result["message"]["text"].is_string());
        let location = &result["locations"][0]["physicalLocation"];
        assert!(location["artifactLocation"]["uri"].is_string());
        let region = &location["region"];
        assert!(region["startLine"].as_u64().expect("startLine") >= 1);
        assert!(region["startColumn"].as_u64().expect("startColumn") >= 1);
    }
    assert!(results[2].get("ruleIndex").is_none());
}

#[test]
fn columns_are_utf16_code_units_and_paths_are_uris() {
    let log: Value = serde_json::from_str(&render()).expect("valid JSON");
    let run = &log["runs"][0];
    assert_eq!(run["columnKind"], "utf16CodeUnits");
    let result = &run["results"][0];
    let location = &result["locations"][0]["physicalLocation"];
    assert_eq!(location["artifactLocation"]["uri"], "deploy/a%20b.yaml");
    assert_eq!(location["artifactLocation"]["uriBaseId"], "%SRCROOT%");
    assert_eq!(location["region"]["startColumn"], 10);
    assert_eq!(location["region"]["endColumn"], 11);
    let related = &result["relatedLocations"][0]["physicalLocation"]["artifactLocation"];
    assert_eq!(related["uri"], "file:///abs/b.yaml");
    assert!(related.get("uriBaseId").is_none());
    assert_eq!(
        result["message"]["text"],
        "duplicate object\nhelp: rename one of them"
    );
}
