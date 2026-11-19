//! Splits rule fixtures into cases and judges each one against what a real API server
//! answered for it.

use std::ops::Range;

use kubelint_linter::{EnabledRule, Linter, Rule, rules};
use kubelint_schema::KubernetesVersion;
use kubelint_source::{SourceFile, TextSize};

/// The comment that takes a case out of the end to end check: `# e2e-skip: <reason>`
/// for every version, `# e2e-skip 1.34: <reason>` for one.
const SKIP_MARKER: &str = "# e2e-skip";

/// A rule judged by Pod Security admission: a case is reported exactly when the server,
/// in a namespace enforcing `level`, names one of `checks`.
#[derive(Debug, Clone, Copy)]
pub struct PodSecurity {
    pub code: &'static str,
    pub level: Level,
    pub checks: &'static [&'static str],
}

/// A Pod Security Standard that a namespace can enforce.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Level {
    Baseline,
    Restricted,
}

impl Level {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Baseline => "baseline",
            Self::Restricted => "restricted",
        }
    }
}

/// The check names are those of `k8s.io/pod-security-admission/policy`.
pub const POD_SECURITY: &[PodSecurity] = &[
    PodSecurity {
        code: "KL2001",
        level: Level::Baseline,
        checks: &["privileged"],
    },
    PodSecurity {
        code: "KL2002",
        level: Level::Restricted,
        checks: &["allowPrivilegeEscalation != false"],
    },
    PodSecurity {
        code: "KL2003",
        level: Level::Restricted,
        checks: &["runAsNonRoot != true", "runAsUser=0"],
    },
    PodSecurity {
        code: "KL2004",
        level: Level::Baseline,
        checks: &["host namespaces"],
    },
    PodSecurity {
        code: "KL2005",
        level: Level::Baseline,
        checks: &["hostPath volumes"],
    },
    PodSecurity {
        code: "KL2006",
        level: Level::Baseline,
        checks: &["non-default capabilities"],
    },
    PodSecurity {
        code: "KL2007",
        level: Level::Baseline,
        checks: &["hostPath volumes"],
    },
];

/// One YAML document of a fixture, with the comment that precedes it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Case {
    /// The 1-based line where the case starts in its fixture.
    pub line: usize,
    pub text: String,
    /// Whether the rule reports anything in this case when it lints the whole fixture.
    pub reported: bool,
    pub skip: Option<String>,
    /// The `metadata.namespace` the case sets, which must exist before it is applied.
    pub namespace: Option<String>,
}

/// What `kubectl apply --dry-run=server` did with a case.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Outcome {
    pub accepted: bool,
    /// Standard output and standard error.
    pub output: String,
}

pub fn rule(code: &str) -> Option<&'static dyn Rule> {
    rules::ALL
        .iter()
        .copied()
        .find(|rule| rule.meta().code == code)
}

/// The fixture of `rule`, relative to the repository root.
pub fn fixture_path(rule: &dyn Rule) -> String {
    let meta = rule.meta();
    format!(
        "crates/kubelint_linter/resources/test/fixtures/{}/{}.yaml",
        meta.category.as_str(),
        meta.code
    )
}

/// Splits `text`, the fixture of `rule`, at its `---` lines and lints it whole with only
/// `rule` enabled, so rules that compare objects see all of them. Fails on a skip marker
/// without a reason or with a version kubelint has no schema for.
pub fn cases(
    rule: &'static dyn Rule,
    text: &str,
    version: KubernetesVersion,
) -> Result<Vec<Case>, String> {
    let file = SourceFile::new(fixture_path(rule), text);
    let findings: Vec<TextSize> = Linter::new(vec![EnabledRule::new(rule)])
        .with_kubernetes_version(version)
        .lint(std::slice::from_ref(&file))
        .diagnostics
        .iter()
        .map(|diagnostic| diagnostic.primary.range.start())
        .collect();
    let mut cases = Vec::new();
    let (mut start, mut line, mut offset) = (0, 1, 0);
    for (index, piece) in text.split_inclusive('\n').enumerate() {
        if piece.trim_end() == "---" {
            cases.push(case(text, start..offset, line, &findings, version)?);
            start = offset + piece.len();
            line = index + 2;
        }
        offset += piece.len();
    }
    cases.push(case(text, start..text.len(), line, &findings, version)?);
    Ok(cases)
}

fn case(
    text: &str,
    range: Range<usize>,
    line: usize,
    findings: &[TextSize],
    version: KubernetesVersion,
) -> Result<Case, String> {
    let body = &text[range.clone()];
    let mut skip = None;
    for (index, marker) in body.lines().enumerate() {
        let Some(rest) = marker.strip_prefix(SKIP_MARKER) else {
            continue;
        };
        let place = line + index;
        let (versions, reason) = rest
            .split_once(':')
            .filter(|(_, reason)| !reason.trim().is_empty())
            .ok_or_else(|| format!("line {place}: write `{SKIP_MARKER}: <reason>`"))?;
        let applies = match versions.trim() {
            "" => true,
            named => {
                KubernetesVersion::parse(named)
                    .ok_or_else(|| format!("line {place}: kubelint has no schema for {named}"))?
                    == version
            }
        };
        if applies {
            skip = Some(reason.trim().to_owned());
        }
    }
    Ok(Case {
        line,
        text: body.to_owned(),
        reported: findings
            .iter()
            .any(|&offset| range.contains(&usize::from(offset))),
        skip,
        namespace: kubelint_yaml::parse(body)
            .documents
            .iter()
            .find_map(|document| {
                document
                    .root
                    .as_ref()?
                    .get_path(&["metadata", "namespace"])?
                    .as_str()
            })
            .map(str::to_owned),
    })
}

/// Why the server disagrees with kubelint on `case`, or `None` when it agrees.
pub fn pod_security_mismatch(rule: &PodSecurity, case: &Case, outcome: &Outcome) -> Option<String> {
    if !outcome.accepted && !outcome.output.contains("violates PodSecurity") {
        return Some("the server rejected the case before Pod Security ran".to_owned());
    }
    let named = rule.checks.iter().any(|check| {
        outcome.output.contains(&format!(": {check} ("))
            || outcome.output.contains(&format!("), {check} ("))
    });
    match (case.reported, named) {
        (true, false) => Some(format!(
            "reported, but Pod Security `{}` does not name {}",
            rule.level.as_str(),
            rule.checks.join(" or ")
        )),
        (false, true) => Some(format!(
            "not reported, but Pod Security `{}` names {}",
            rule.level.as_str(),
            rule.checks.join(" or ")
        )),
        _ => None,
    }
}
