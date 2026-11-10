//! Table-driven tests of the image reference splitter.

use kubelint_k8s::ImageRef;

#[test]
fn image_references() {
    let cases: &[(&str, &str, Option<&str>, Option<&str>)] = &[
        ("nginx", "nginx", None, None),
        ("nginx:1.27", "nginx", Some("1.27"), None),
        ("nginx:latest", "nginx", Some("latest"), None),
        ("library/nginx:1.27", "library/nginx", Some("1.27"), None),
        (
            "docker.io/library/nginx",
            "docker.io/library/nginx",
            None,
            None,
        ),
        ("ghcr.io/org/app:v2", "ghcr.io/org/app", Some("v2"), None),
        ("localhost:5000/app", "localhost:5000/app", None, None),
        (
            "localhost:5000/app:1.0",
            "localhost:5000/app",
            Some("1.0"),
            None,
        ),
        (
            "host:5000/team/app:1.0",
            "host:5000/team/app",
            Some("1.0"),
            None,
        ),
        ("[::1]:5000/app", "[::1]:5000/app", None, None),
        ("[::1]:5000/app:1.0", "[::1]:5000/app", Some("1.0"), None),
        ("app@sha256:abc", "app", None, Some("sha256:abc")),
        ("app:1.0@sha256:abc", "app", Some("1.0"), Some("sha256:abc")),
        (
            "host:5000/app@sha256:abc",
            "host:5000/app",
            None,
            Some("sha256:abc"),
        ),
        ("app:", "app", Some(""), None),
        ("", "", None, None),
    ];
    for &(image, name, tag, digest) in cases {
        assert_eq!(
            ImageRef::parse(image),
            ImageRef { name, tag, digest },
            "{image}"
        );
    }
}

#[test]
fn repository_paths() {
    let cases: &[(&str, &str)] = &[
        ("nginx", "nginx"),
        ("library/nginx", "library/nginx"),
        ("docker.io/library/nginx:1", "library/nginx"),
        ("localhost/app", "app"),
        ("localhost:5000/team/app", "team/app"),
        ("[::1]:5000/app", "app"),
        ("Registry/App", "App"),
        ("MyOrg", "MyOrg"),
    ];
    for &(image, path) in cases {
        assert_eq!(ImageRef::parse(image).path(), path, "{image}");
    }
}
