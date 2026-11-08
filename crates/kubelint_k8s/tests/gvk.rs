use kubelint_k8s::{BuiltinKind, GroupVersion, stable_api_version};

#[test]
fn parses_api_versions() {
    let cases = [
        ("v1", Some(("", "v1"))),
        ("apps/v1", Some(("apps", "v1"))),
        ("networking.k8s.io/v1", Some(("networking.k8s.io", "v1"))),
        ("batch/v1beta1", Some(("batch", "v1beta1"))),
        ("", None),
        ("apps/", None),
        ("/v1", None),
        ("a/b/c", None),
        ("/", None),
    ];
    for (input, expected) in cases {
        let got = GroupVersion::parse(input).map(|gv| (gv.group, gv.version));
        assert_eq!(got, expected, "input {input:?}");
    }
}

#[test]
fn display_round_trips() {
    for input in ["v1", "apps/v1", "rbac.authorization.k8s.io/v1"] {
        let gv = GroupVersion::parse(input).map(|gv| gv.to_string());
        assert_eq!(gv.as_deref(), Some(input));
    }
}

#[test]
fn builtin_kinds() {
    let cases = [
        ("Pod", BuiltinKind::Pod, "v1", ""),
        ("StatefulSet", BuiltinKind::StatefulSet, "apps/v1", "apps"),
        ("CronJob", BuiltinKind::CronJob, "batch/v1", "batch"),
        (
            "HorizontalPodAutoscaler",
            BuiltinKind::HorizontalPodAutoscaler,
            "autoscaling/v2",
            "autoscaling",
        ),
        (
            "CSIDriver",
            BuiltinKind::CSIDriver,
            "storage.k8s.io/v1",
            "storage.k8s.io",
        ),
        (
            "ValidatingWebhookConfiguration",
            BuiltinKind::ValidatingWebhookConfiguration,
            "admissionregistration.k8s.io/v1",
            "admissionregistration.k8s.io",
        ),
    ];
    for (name, kind, api_version, group) in cases {
        assert_eq!(BuiltinKind::parse(name), Some(kind));
        assert_eq!(
            (kind.as_str(), kind.api_version(), kind.group()),
            (name, api_version, group)
        );
        assert_eq!(stable_api_version(name), Some(api_version));
    }
}

#[test]
fn other_kinds_are_not_builtin() {
    for name in [
        "pod",
        "STATEFULSET",
        "CsiDriver",
        "Certificate",
        "List",
        "",
        " Pod",
    ] {
        assert_eq!(BuiltinKind::parse(name), None, "{name:?}");
        assert_eq!(stable_api_version(name), None, "{name:?}");
    }
}
