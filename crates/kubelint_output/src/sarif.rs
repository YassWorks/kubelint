//! The SARIF renderer: SARIF 2.1.0 for GitHub code scanning and other dashboards.
//!
//! Only the parts of SARIF that kubelint needs are modelled, with hand-written `serde`
//! structs. The mapping is documented in `docs/user/output-formats.md`.

use std::fmt::Write as _;

use kubelint_diagnostics::{Diagnostic, Label, Severity};
use kubelint_source::{SourceFile, TextSize};
use serde::Serialize;

use crate::human::path_string;

/// What the SARIF output says about one rule. The renderer cannot read the rule registry
/// itself (it lives in a higher layer), so the caller describes every rule.
#[derive(Debug, Clone, Copy)]
pub struct SarifRule<'a> {
    /// The rule code, such as `KL2001`.
    pub code: &'a str,
    /// The rule name, such as `privileged-container`.
    pub name: &'a str,
    /// One line describing the problem.
    pub summary: &'a str,
    /// Markdown: why it matters and how to fix it.
    pub explanation: &'a str,
    /// The category name, such as `security`.
    pub category: &'a str,
    /// The severity the rule reports at unless configured otherwise.
    pub default_severity: Severity,
}

/// Renders diagnostics as a SARIF 2.1.0 log with one run.
#[derive(Debug, Clone, Copy)]
pub struct SarifRenderer<'a> {
    version: &'a str,
    rules: &'a [SarifRule<'a>],
}

impl<'a> SarifRenderer<'a> {
    /// A renderer describing kubelint `version` with these `rules`. Every rule is listed
    /// in the output, whether or not it found anything, so rule indexes are stable.
    pub fn new(version: &'a str, rules: &'a [SarifRule<'a>]) -> Self {
        Self { version, rules }
    }

    /// Renders every diagnostic as a SARIF log, pretty-printed and ending with a newline.
    /// `files` are the files of the run, indexed by [`FileId`](kubelint_source::FileId).
    ///
    /// ```
    /// use kubelint_diagnostics::{Diagnostic, Label, Severity};
    /// use kubelint_output::{SarifRenderer, SarifRule};
    /// use kubelint_source::{FileId, SourceFile, TextRange};
    ///
    /// let rules = [SarifRule {
    ///     code: "KL1001",
    ///     name: "missing-api-version",
    ///     summary: "The object has no `apiVersion`.",
    ///     explanation: "Every object needs an `apiVersion`.",
    ///     category: "correctness",
    ///     default_severity: Severity::Error,
    /// }];
    /// let files = [SourceFile::new("pod.yaml", "kind: Pod\n")];
    /// let range = TextRange::new(0.into(), 9.into());
    /// let diagnostics = [Diagnostic::new(
    ///     "KL1001",
    ///     Severity::Error,
    ///     "`Pod` object has no `apiVersion`",
    ///     Label::new(FileId(0), range),
    /// )];
    /// let out = SarifRenderer::new("0.1.0", &rules).render(&files, &diagnostics);
    /// let log: serde_json::Value = serde_json::from_str(&out).unwrap();
    /// assert_eq!(log["version"], "2.1.0");
    /// assert_eq!(log["runs"][0]["results"][0]["ruleId"], "KL1001");
    /// ```
    pub fn render(&self, files: &[SourceFile], diagnostics: &[Diagnostic]) -> String {
        let log = Log {
            schema: SCHEMA,
            version: "2.1.0",
            runs: [Run {
                tool: Tool {
                    driver: Driver {
                        name: "kubelint",
                        version: self.version,
                        information_uri: INFORMATION_URI,
                        rules: self.rules.iter().map(reporting_descriptor).collect(),
                    },
                },
                column_kind: "utf16CodeUnits",
                results: diagnostics.iter().map(|d| self.result(files, d)).collect(),
            }],
        };
        // Serializing plain structs of strings and integers cannot fail.
        let mut out = serde_json::to_string_pretty(&log).unwrap_or_default();
        out.push('\n');
        out
    }

    fn result<'d>(&self, files: &[SourceFile], diagnostic: &'d Diagnostic) -> SarifResult<'d> {
        let text = match &diagnostic.help {
            Some(help) => format!("{}\nhelp: {help}", diagnostic.message),
            None => diagnostic.message.clone(),
        };
        SarifResult {
            rule_id: diagnostic.code,
            rule_index: self.rules.iter().position(|r| r.code == diagnostic.code),
            level: level(diagnostic.severity),
            message: Message { text },
            locations: vec![location(files, &diagnostic.primary)],
            related_locations: diagnostic
                .secondary
                .iter()
                .map(|label| location(files, label))
                .collect(),
        }
    }
}

const SCHEMA: &str = "https://json.schemastore.org/sarif-2.1.0.json";
const INFORMATION_URI: &str = "https://github.com/YassWorks/kubelint";

fn level(severity: Severity) -> &'static str {
    match severity {
        Severity::Error => "error",
        Severity::Warning => "warning",
        Severity::Info => "note",
    }
}

fn reporting_descriptor<'a>(rule: &SarifRule<'a>) -> ReportingDescriptor<'a> {
    ReportingDescriptor {
        id: rule.code,
        name: rule.name,
        short_description: Text { text: rule.summary },
        full_description: Text { text: rule.summary },
        help: Help {
            text: rule.explanation,
            markdown: rule.explanation,
        },
        default_configuration: Configuration {
            level: level(rule.default_severity),
        },
        properties: Properties {
            tags: [rule.category],
        },
    }
}

fn location<'a>(files: &[SourceFile], label: &'a Label) -> Location<'a> {
    let file = files.get(label.file.0 as usize);
    let region = file.map(|file| {
        let position = |offset: TextSize| file.line_index().line_col(file.text(), offset);
        let (start, end) = (position(label.range.start()), position(label.range.end()));
        Region {
            start_line: start.line,
            start_column: start.utf16_col,
            end_line: end.line,
            end_column: end.utf16_col,
        }
    });
    let path = file.map(path_string).unwrap_or_default();
    Location {
        physical_location: PhysicalLocation {
            artifact_location: ArtifactLocation {
                uri_base_id: is_relative(&path).then_some("%SRCROOT%"),
                uri: uri(&path),
            },
            region,
        },
        message: label.message.as_deref().map(|text| Text { text }),
    }
}

/// Whether a `/`-separated path is relative (not `/x`, not `C:/x`).
fn is_relative(path: &str) -> bool {
    let bytes = path.as_bytes();
    let drive = bytes.len() >= 3 && bytes[0].is_ascii_alphabetic() && &bytes[1..3] == b":/";
    !path.starts_with('/') && !drive
}

/// A URI reference for a `/`-separated path: relative paths stay relative, absolute ones
/// become `file:` URIs, and characters outside the URI "unreserved" set are
/// percent-encoded byte by byte.
fn uri(path: &str) -> String {
    let absolute = !is_relative(path);
    let mut out = String::with_capacity(path.len());
    if absolute {
        out.push_str(if path.starts_with('/') {
            "file://"
        } else {
            "file:///"
        });
    }
    for byte in path.bytes() {
        // A colon is only kept in an absolute path, where it ends a Windows drive letter;
        // in a relative path it would be read as a URI scheme.
        if byte.is_ascii_alphanumeric() || b"-._~/".contains(&byte) || (byte == b':' && absolute) {
            out.push(char::from(byte));
        } else {
            let _ = write!(out, "%{byte:02X}");
        }
    }
    out
}

#[derive(Serialize)]
struct Log<'a> {
    #[serde(rename = "$schema")]
    schema: &'static str,
    version: &'static str,
    runs: [Run<'a>; 1],
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Run<'a> {
    tool: Tool<'a>,
    column_kind: &'static str,
    results: Vec<SarifResult<'a>>,
}

#[derive(Serialize)]
struct Tool<'a> {
    driver: Driver<'a>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Driver<'a> {
    name: &'static str,
    version: &'a str,
    information_uri: &'static str,
    rules: Vec<ReportingDescriptor<'a>>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ReportingDescriptor<'a> {
    id: &'a str,
    name: &'a str,
    short_description: Text<'a>,
    full_description: Text<'a>,
    help: Help<'a>,
    default_configuration: Configuration,
    properties: Properties<'a>,
}

#[derive(Serialize)]
struct Text<'a> {
    text: &'a str,
}

#[derive(Serialize)]
struct Message {
    text: String,
}

#[derive(Serialize)]
struct Help<'a> {
    text: &'a str,
    markdown: &'a str,
}

#[derive(Serialize)]
struct Configuration {
    level: &'static str,
}

#[derive(Serialize)]
struct Properties<'a> {
    tags: [&'a str; 1],
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SarifResult<'a> {
    rule_id: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    rule_index: Option<usize>,
    level: &'static str,
    message: Message,
    locations: Vec<Location<'a>>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    related_locations: Vec<Location<'a>>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Location<'a> {
    physical_location: PhysicalLocation,
    #[serde(skip_serializing_if = "Option::is_none")]
    message: Option<Text<'a>>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct PhysicalLocation {
    artifact_location: ArtifactLocation,
    #[serde(skip_serializing_if = "Option::is_none")]
    region: Option<Region>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ArtifactLocation {
    uri: String,
    #[serde(rename = "uriBaseId", skip_serializing_if = "Option::is_none")]
    uri_base_id: Option<&'static str>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Region {
    start_line: u32,
    start_column: u32,
    end_line: u32,
    end_column: u32,
}

#[cfg(test)]
mod tests {
    use super::{is_relative, uri};

    #[test]
    fn uris() {
        assert_eq!(uri("deploy/web.yaml"), "deploy/web.yaml");
        assert_eq!(uri("my dir/é.yaml"), "my%20dir/%C3%A9.yaml");
        assert_eq!(uri("<stdin>"), "%3Cstdin%3E");
        assert_eq!(uri("a:b.yaml"), "a%3Ab.yaml");
        assert_eq!(uri("/srv/app/x.yaml"), "file:///srv/app/x.yaml");
        assert_eq!(uri("C:/work/x.yaml"), "file:///C:/work/x.yaml");
        assert!(is_relative("./x.yaml"));
        assert!(!is_relative("D:/x"));
    }
}
