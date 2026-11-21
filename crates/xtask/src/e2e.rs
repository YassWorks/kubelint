//! Splits rule fixtures into cases and judges each one against what a real API server
//! answered for it.

use std::ops::Range;

use kubelint_linter::{EnabledRule, Linter, Rule, rules};
use kubelint_schema::KubernetesVersion;
use kubelint_source::{SourceFile, TextSize};

/// The comment that takes a case out of the end to end check: `# e2e-skip: <reason>`
/// for every version, `# e2e-skip 1.34: <reason>` for one.
const SKIP_MARKER: &str = "# e2e-skip";

/// A rule and the server answers that mean the rule's field is wrong: a case is reported
/// exactly when the server gives one of `needles`. A needle matches within one line of
/// the server's output, with every list index written `[]`, and `*` matches any text.
#[derive(Debug, Clone, Copy)]
pub struct Check {
    pub code: &'static str,
    pub request: Request,
    pub stage: Stage,
    pub needles: &'static [&'static str],
}

/// The `kubectl` command that sends a case to the server as a dry run.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Request {
    Apply,
    /// For cases without a `name`, which `kubectl apply` refuses before it sends them.
    Create,
}

impl Request {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Apply => "apply",
            Self::Create => "create",
        }
    }
}

/// Where the server judges a rule's field. The server decodes an object, validates it and
/// then runs admission, and a case it rejects before that stage proves nothing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Stage {
    /// Decoding reports one error, and a validation error shows that decoding passed.
    Decoding,
    /// Validation reports every invalid field, except that an invalid workload selector
    /// skips the checks of the pod template.
    Validation,
    /// Validation warnings come only with an object that passed validation.
    Warning,
    /// Pod Security admission, in namespaces that enforce the level. The needles are check
    /// names of `k8s.io/pod-security-admission/policy`.
    PodSecurity(Level),
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

pub const CHECKS: &[Check] = &[
    Check {
        code: "KL1003",
        request: Request::Create,
        stage: Stage::Validation,
        needles: &["metadata.name: Required value: name or generateName is required"],
    },
    Check {
        code: "KL1005",
        request: Request::Apply,
        stage: Stage::Validation,
        needles: &["`selector` does not match template `labels`"],
    },
    Check {
        code: "KL1006",
        request: Request::Apply,
        stage: Stage::Validation,
        needles: &[
            "containers[].name: Duplicate value",
            "Containers[].name: Duplicate value",
        ],
    },
    Check {
        code: "KL1007",
        request: Request::Apply,
        stage: Stage::Decoding,
        needles: &["quantities must match the regular expression"],
    },
    Check {
        code: "KL1008",
        request: Request::Create,
        stage: Stage::Validation,
        needles: &[
            "metadata.name: Invalid value",
            "metadata.generateName: Invalid value",
            "namespaces \"*\" not found",
            "spec.template.labels: Invalid value: \"*\": must be no more than 63",
        ],
    },
    Check {
        code: "KL1009",
        request: Request::Apply,
        stage: Stage::Decoding,
        needles: &[
            "cannot unmarshal number into Go struct field EnvVar.*.env.value of type string",
            "cannot unmarshal bool into Go struct field EnvVar.*.env.value of type string",
        ],
    },
    Check {
        code: "KL1010",
        request: Request::Apply,
        stage: Stage::Validation,
        needles: &[
            "labels: Invalid value: \"",
            "matchLabels: Invalid value: \"",
            "selector: Invalid value: \"",
        ],
    },
    Check {
        code: "KL1011",
        request: Request::Apply,
        stage: Stage::Warning,
        needles: &["hides previous definition of"],
    },
    Check {
        code: "KL2001",
        request: Request::Apply,
        stage: Stage::PodSecurity(Level::Baseline),
        needles: &["privileged"],
    },
    Check {
        code: "KL2002",
        request: Request::Apply,
        stage: Stage::PodSecurity(Level::Restricted),
        needles: &["allowPrivilegeEscalation != false"],
    },
    Check {
        code: "KL2003",
        request: Request::Apply,
        stage: Stage::PodSecurity(Level::Restricted),
        needles: &["runAsNonRoot != true", "runAsUser=0"],
    },
    Check {
        code: "KL2004",
        request: Request::Apply,
        stage: Stage::PodSecurity(Level::Baseline),
        needles: &["host namespaces"],
    },
    Check {
        code: "KL2005",
        request: Request::Apply,
        stage: Stage::PodSecurity(Level::Baseline),
        needles: &["hostPath volumes"],
    },
    Check {
        code: "KL2006",
        request: Request::Apply,
        stage: Stage::PodSecurity(Level::Baseline),
        needles: &["non-default capabilities"],
    },
    Check {
        code: "KL2007",
        request: Request::Apply,
        stage: Stage::PodSecurity(Level::Baseline),
        needles: &["hostPath volumes"],
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
    /// The `metadata.namespace` of the case and of its `kind: List` items, which must
    /// exist before it is sent.
    pub namespaces: Vec<String>,
}

/// What the check's `kubectl` dry run did with a case.
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
        namespaces: kubelint_yaml::parse(body)
            .documents
            .iter()
            .filter_map(|document| document.root.as_ref())
            .flat_map(|root| {
                std::iter::once(root).chain(
                    root.get("items")
                        .and_then(kubelint_yaml::Node::as_sequence)
                        .unwrap_or_default(),
                )
            })
            .filter_map(|object| object.get_path(&["metadata", "namespace"])?.as_str())
            .map(str::to_owned)
            .collect(),
    })
}

/// Why the server disagrees with kubelint on `case`, or `None` when it agrees.
pub fn mismatch(check: &Check, case: &Case, outcome: &Outcome) -> Option<String> {
    let output = without_indices(&outcome.output);
    let named = check.needles.iter().any(|needle| match check.stage {
        Stage::PodSecurity(_) => {
            output.contains(&format!(": {needle} (")) || output.contains(&format!("), {needle} ("))
        }
        _ => output.lines().any(|line| matches(line, needle)),
    });
    let validated = output.contains(" is invalid:");
    let judged = outcome.accepted
        || named
        || match check.stage {
            Stage::Decoding => validated,
            Stage::Validation => validated && !output.contains("invalid label selector"),
            Stage::PodSecurity(_) => output.contains("violates PodSecurity"),
            Stage::Warning => false,
        };
    let needles = check.needles.join(" or ");
    match (judged, case.reported, named) {
        (false, ..) => {
            Some("the server rejected the case before it checked the rule's field".to_owned())
        }
        (true, true, false) => Some(format!("reported, but the server does not say {needles}")),
        (true, false, true) => Some(format!("not reported, but the server says {needles}")),
        _ => None,
    }
}

fn without_indices(output: &str) -> String {
    let mut out = String::with_capacity(output.len());
    let mut rest = output;
    while let Some(open) = rest.find('[') {
        let (before, after) = rest.split_at(open + 1);
        out.push_str(before);
        let digits = after.bytes().take_while(u8::is_ascii_digit).count();
        rest = if digits > 0 && after[digits..].starts_with(']') {
            &after[digits..]
        } else {
            after
        };
    }
    out.push_str(rest);
    out
}

fn matches(line: &str, needle: &str) -> bool {
    let mut rest = line;
    needle.split('*').all(|part| match rest.find(part) {
        Some(at) => {
            rest = &rest[at + part.len()..];
            true
        }
        None => false,
    })
}
