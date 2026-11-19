use kubelint_schema::{KubernetesVersion, VERSIONS};
use xtask::e2e::{self, Case, Outcome, POD_SECURITY};

const FIXTURE: &str = include_str!("../resources/test/e2e/KL2001.yaml");

fn version(text: &str) -> KubernetesVersion {
    KubernetesVersion::parse(text).expect("a supported version")
}

fn cases(text: &str) -> Vec<Case> {
    let rule = e2e::rule("KL2001").expect("KL2001 exists");
    e2e::cases(rule, FIXTURE, version(text))
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
fn judges_pod_security_answers() {
    let rule = POD_SECURITY
        .iter()
        .find(|rule| rule.code == "KL2003")
        .expect("KL2003 is checked");
    let case = |reported| Case {
        line: 1,
        text: String::new(),
        reported,
        skip: None,
        namespace: None,
    };
    let outcome = |accepted, output: &str| Outcome {
        accepted,
        output: output.to_owned(),
    };
    let rejected = outcome(
        false,
        r#"pods "a" is forbidden: violates PodSecurity "restricted:latest": seccompProfile (x), runAsUser=0 (container "a" must not set runAsUser=0)"#,
    );
    let warned = outcome(
        true,
        r#"Warning: would violate PodSecurity "restricted:latest": runAsNonRoot != true (pod or container "a" must set securityContext.runAsNonRoot=true)"#,
    );
    let other_check = outcome(
        true,
        r#"Warning: would violate PodSecurity "restricted:latest": seccompProfile (pod or container "a" must set securityContext.seccompProfile.type)"#,
    );
    let invalid = outcome(
        false,
        r#"The Pod "a" is invalid: spec.containers[0].name: Required value"#,
    );
    assert_eq!(
        e2e::pod_security_mismatch(rule, &case(true), &rejected),
        None
    );
    assert_eq!(e2e::pod_security_mismatch(rule, &case(true), &warned), None);
    assert_eq!(
        e2e::pod_security_mismatch(rule, &case(false), &other_check),
        None
    );
    assert_eq!(
        e2e::pod_security_mismatch(rule, &case(true), &other_check).as_deref(),
        Some(
            "reported, but Pod Security `restricted` does not name runAsNonRoot != true or runAsUser=0"
        )
    );
    assert_eq!(
        e2e::pod_security_mismatch(rule, &case(false), &warned).as_deref(),
        Some(
            "not reported, but Pod Security `restricted` names runAsNonRoot != true or runAsUser=0"
        )
    );
    assert_eq!(
        e2e::pod_security_mismatch(rule, &case(false), &invalid).as_deref(),
        Some("the server rejected the case before Pod Security ran")
    );
}

#[test]
fn every_checked_rule_exists() {
    for rule in POD_SECURITY {
        assert!(e2e::rule(rule.code).is_some(), "{}", rule.code);
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
