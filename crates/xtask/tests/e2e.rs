use kubelint_schema::{KubernetesVersion, VERSIONS};
use xtask::e2e::{self, CHECKS, Case, Outcome};

const FIXTURE: &str = include_str!("../resources/test/e2e/KL2001.yaml");

fn version(text: &str) -> KubernetesVersion {
    KubernetesVersion::parse(text).expect("a supported version")
}

fn cases(text: &str) -> Vec<Case> {
    let rule = e2e::rule("KL2001").expect("KL2001 exists");
    e2e::cases(rule, FIXTURE, version(text)).expect("valid markers")
}

#[test]
fn splits_a_fixture_into_cases_with_their_verdict() {
    let cases = cases("1.37");
    let summary: Vec<_> = cases
        .iter()
        .map(|case| {
            (
                case.line,
                case.reported,
                case.skip.as_deref(),
                case.namespace.as_deref(),
            )
        })
        .collect();
    assert_eq!(
        summary,
        [
            (1, true, None, Some("team-a")),
            (11, false, Some("a custom resource"), None),
            (18, false, None, None),
        ]
    );
    assert!(
        cases[1]
            .text
            .starts_with("# Not reported, skipped on every version.\n")
    );
    assert!(cases[0].text.ends_with("privileged: true}}\n"));
}

#[test]
fn a_version_marker_skips_only_that_version() {
    assert_eq!(cases("1.34")[2].skip.as_deref(), Some("differs on 1.34"));
    assert_eq!(cases("1.35")[2].skip, None);
}

#[test]
fn rejects_markers_without_a_reason_or_with_an_unknown_version() {
    let rule = e2e::rule("KL2001").expect("KL2001 exists");
    let latest = KubernetesVersion::LATEST;
    for (text, error) in [
        (
            "# e2e-skip 1.73: old\na: 1\n",
            "line 1: kubelint has no schema for 1.73",
        ),
        (
            "a: 1\n# e2e-skip no colon\n",
            "line 2: write `# e2e-skip: <reason>`",
        ),
        (
            "# e2e-skip:\na: 1\n",
            "line 1: write `# e2e-skip: <reason>`",
        ),
    ] {
        assert_eq!(e2e::cases(rule, text, latest).err().as_deref(), Some(error));
    }
}

const BEFORE: Option<&str> =
    Some("the server rejected the case before it checked the rule's field");
const NON_ROOT: &str = r#"Warning: would violate PodSecurity "restricted:latest": runAsNonRoot != true (pod or container "a" must set securityContext.runAsNonRoot=true)"#;
const SECCOMP: &str = r#"Warning: would violate PodSecurity "restricted:latest": seccompProfile (pod or container "a" must set securityContext.seccompProfile.type)"#;
const FORBIDDEN: &str = r#"pods "a" is forbidden: violates PodSecurity "restricted:latest": seccompProfile (x), runAsUser=0 (container "a" must not set runAsUser=0)"#;
const NAMELESS: &str = r#"The Pod "a" is invalid: spec.containers[0].name: Required value"#;
const QUANTITY: &str = r#"Pod in version "v1" cannot be handled as a Pod: quantities must match the regular expression '^x$'"#;
const ARRAY: &str = r#"Pod in version "v1" cannot be handled as a Pod: json: cannot unmarshal array into Go struct field Container.spec.containers.name of type string"#;
const ENV_NAME: &str = r#"Pod in version "v1" cannot be handled as a Pod: json: cannot unmarshal bool into Go struct field EnvVar.spec.containers.env.name of type string"#;
const ENV_VALUE: &str = r#"Pod in version "v1" cannot be handled as a Pod: json: cannot unmarshal number into Go struct field EnvVar.spec.containers.env.value of type string"#;
const DUPLICATE: &str =
    r#"The Pod "a" is invalid: spec.initContainers[12].name: Duplicate value: "a""#;
const DUPLICATE_VOLUME: &str =
    r#"The Pod "a" is invalid: spec.volumes[1].name: Duplicate value: "a""#;
const DUPLICATE_PORT: &str =
    r#"The Pod "a" is invalid: spec.containers[0].ports[1].name: Duplicate value: "http""#;
const MISMATCH: &str = r#"The Deployment "a" is invalid: spec.template.metadata.labels: Invalid value: {"app":"a"}: `selector` does not match template `labels`"#;
const BAD_SELECTOR: &str = r#"The Deployment "a" is invalid: spec.selector: Invalid value: {"matchExpressions":[{"key":"k","operator":"In"}]}: invalid label selector"#;
const HIDES: &str = r#"Warning: spec.containers[0].env[1]: hides previous definition of "A", which may be dropped when using apply"#;
const CREATED: &str = "pod/a created (server dry run)";

/// Rule, reported, accepted, server output, expected mismatch.
const ANSWERS: &[(&str, bool, bool, &str, Option<&str>)] = &[
    ("KL2003", true, false, FORBIDDEN, None),
    ("KL2003", true, true, NON_ROOT, None),
    ("KL2003", false, true, SECCOMP, None),
    ("KL2003", false, true, CREATED, None),
    (
        "KL2003",
        true,
        true,
        SECCOMP,
        Some("reported, but the server does not say runAsNonRoot != true or runAsUser=0"),
    ),
    (
        "KL2003",
        true,
        true,
        CREATED,
        Some("reported, but the server does not say runAsNonRoot != true or runAsUser=0"),
    ),
    (
        "KL2003",
        false,
        true,
        NON_ROOT,
        Some("not reported, but the server says runAsNonRoot != true or runAsUser=0"),
    ),
    ("KL2003", false, false, NAMELESS, BEFORE),
    ("KL1007", true, false, QUANTITY, None),
    ("KL1007", false, false, NAMELESS, None),
    ("KL1007", false, false, ARRAY, BEFORE),
    (
        "KL1007",
        false,
        false,
        QUANTITY,
        Some("not reported, but the server says quantities must match the regular expression"),
    ),
    ("KL1006", false, false, NAMELESS, None),
    ("KL1006", true, false, DUPLICATE, None),
    ("KL1006", false, false, DUPLICATE_VOLUME, None),
    ("KL1006", false, false, DUPLICATE_PORT, None),
    (
        "KL1006",
        true,
        false,
        DUPLICATE_PORT,
        Some(
            "reported, but the server does not say containers[].name: Duplicate value or Containers[].name: Duplicate value",
        ),
    ),
    ("KL1005", true, false, MISMATCH, None),
    ("KL1005", false, false, NAMELESS, None),
    (
        "KL1005",
        false,
        false,
        MISMATCH,
        Some("not reported, but the server says `selector` does not match template `labels`"),
    ),
    ("KL1005", false, false, BAD_SELECTOR, BEFORE),
    ("KL1009", true, false, ENV_VALUE, None),
    ("KL1009", true, false, ENV_NAME, BEFORE),
    ("KL1009", false, false, ENV_NAME, BEFORE),
    ("KL1006", false, false, ARRAY, BEFORE),
    ("KL1011", true, true, HIDES, None),
    ("KL1011", false, true, CREATED, None),
    ("KL1011", false, false, NAMELESS, BEFORE),
];

#[test]
fn judges_server_answers() {
    for &(code, reported, accepted, output, expected) in ANSWERS {
        let check = CHECKS
            .iter()
            .find(|check| check.code == code)
            .expect("the rule is checked");
        let case = Case {
            line: 1,
            text: String::new(),
            reported,
            skip: None,
            namespace: None,
        };
        let outcome = Outcome {
            accepted,
            output: output.to_owned(),
        };
        assert_eq!(
            e2e::mismatch(check, &case, &outcome).as_deref(),
            expected,
            "{code} reported={reported} on {output}"
        );
    }
}

#[test]
fn every_checked_rule_exists() {
    for check in CHECKS {
        assert!(e2e::rule(check.code).is_some(), "{}", check.code);
    }
}

#[test]
fn ci_checks_every_supported_version() {
    let workflow = include_str!("../../../.github/workflows/ci.yml");
    let expected = format!(
        "kubernetes: [{}]",
        VERSIONS
            .iter()
            .map(|version| format!("\"{version}\""))
            .collect::<Vec<_>>()
            .join(", ")
    );
    assert!(workflow.contains(&expected), "ci.yml must list {expected}");
}
