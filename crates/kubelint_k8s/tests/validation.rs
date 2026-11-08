use kubelint_k8s::{
    Format, KeyPart, ValidationError, ValidationErrorKind, is_dns1035_label, is_dns1123_label,
    is_dns1123_subdomain, is_qualified_name, is_valid_label_value,
};

fn format(format: Format) -> ValidationError {
    ValidationError {
        part: None,
        kind: ValidationErrorKind::Format(format),
    }
}

fn too_long(max: usize, part: Option<KeyPart>) -> ValidationError {
    ValidationError {
        part,
        kind: ValidationErrorKind::TooLong { max },
    }
}

// The good and bad values below come from Kubernetes' `validation_test.go`.

const LABEL_GOOD: &[&str] = &[
    "a",
    "ab",
    "abc",
    "a1",
    "a-1",
    "a--1--2--b",
    "0",
    "01",
    "012",
    "1a",
    "1-a",
    "1--a--b--2",
];

const LABEL_BAD: &[&str] = &[
    "", "A", "ABC", "aBc", "A1", "A-1", "1-A", "-", "a-", "-a", "1-", "-1", "_", "a_", "_a", "a_b",
    "1_", "_1", "1_2", ".", "a.", ".a", "a.b", "1.", ".1", "1.2", " ", "a ", " a", "a b", "1 ",
    " 1", "1 2",
];

const SUBDOMAIN_GOOD: &[&str] = &[
    "a",
    "ab",
    "abc",
    "a1",
    "a-1",
    "a--1--2--b",
    "0",
    "01",
    "012",
    "1a",
    "1-a",
    "1--a--b--2",
    "a.a",
    "ab.a",
    "abc.a",
    "a1.a",
    "a-1.a",
    "a--1--2--b.a",
    "a.1",
    "ab.1",
    "abc.1",
    "a1.1",
    "a-1.1",
    "a--1--2--b.1",
    "0.a",
    "01.a",
    "012.a",
    "1a.a",
    "1-a.a",
    "1--a--b--2",
    "0.1",
    "01.1",
    "012.1",
    "1a.1",
    "1-a.1",
    "1--a--b--2.1",
    "a.b.c.d.e",
    "aa.bb.cc.dd.ee",
    "1.2.3.4.5",
    "11.22.33.44.55",
];

const SUBDOMAIN_BAD: &[&str] = &[
    "",
    "A",
    "ABC",
    "aBc",
    "A1",
    "A-1",
    "1-A",
    "-",
    "a-",
    "-a",
    "1-",
    "-1",
    "_",
    "a_",
    "_a",
    "a_b",
    "1_",
    "_1",
    "1_2",
    ".",
    "a.",
    ".a",
    "a..b",
    "1.",
    ".1",
    "1..2",
    " ",
    "a ",
    " a",
    "a b",
    "1 ",
    " 1",
    "1 2",
    "A.a",
    "aB.a",
    "ab.A",
    "A1.a",
    "a1.A",
    "A.1",
    "aB.1",
    "A1.1",
    "1A.1",
    "0.A",
    "01.A",
    "012.A",
    "1A.a",
    "1a.A",
    "A.B.C.D.E",
    "AA.BB.CC.DD.EE",
    "a.B.c.d.e",
    "aa.bB.cc.dd.ee",
    "a@b",
    "a,b",
    "a_b",
    "a;b",
    "a:b",
    "a%b",
    "a?b",
    "a$b",
];

const DNS1035_BAD: &[&str] = &[
    "0",
    "01",
    "123",
    "1a",
    "1-a",
    "1--a--b--2",
    "",
    "A",
    "ABC",
    "aBc",
    "A1",
    "A-1",
    "1-A",
    "-",
    "a-",
    "-a",
    "1-",
    "-1",
    "_",
    "a_",
    "_a",
    "a_b",
    "1_",
    "_1",
    "1_2",
    ".",
    "a.",
    ".a",
    "a.b",
    "1.",
    ".1",
    "1.2",
    " ",
    "a ",
    " a",
    "a b",
    "1 ",
    " 1",
    "1 2",
];

#[test]
fn dns1123_labels() {
    for value in LABEL_GOOD.iter().copied().chain([&*"a".repeat(63)]) {
        assert_eq!(is_dns1123_label(value), Ok(()), "{value}");
    }
    for value in LABEL_BAD.iter().copied() {
        assert_eq!(
            is_dns1123_label(value),
            Err(format(Format::Dns1123Label)),
            "{value:?}"
        );
    }
    assert_eq!(is_dns1123_label(&"a".repeat(64)), Err(too_long(63, None)));
}

#[test]
fn dns1123_subdomains() {
    for value in SUBDOMAIN_GOOD.iter().copied().chain([&*"a".repeat(253)]) {
        assert_eq!(is_dns1123_subdomain(value), Ok(()), "{value}");
    }
    for value in SUBDOMAIN_BAD.iter().copied() {
        assert_eq!(
            is_dns1123_subdomain(value),
            Err(format(Format::Dns1123Subdomain)),
            "{value:?}"
        );
    }
    assert_eq!(
        is_dns1123_subdomain(&"a".repeat(254)),
        Err(too_long(253, None))
    );
}

#[test]
fn dns1035_labels() {
    for value in ["a", "ab", "abc", "a1", "a-1", "a--1--2--b"] {
        assert_eq!(is_dns1035_label(value), Ok(()), "{value}");
    }
    assert_eq!(is_dns1035_label(&"a".repeat(63)), Ok(()));
    for value in DNS1035_BAD.iter().copied() {
        assert_eq!(
            is_dns1035_label(value),
            Err(format(Format::Dns1035Label)),
            "{value:?}"
        );
    }
    assert_eq!(is_dns1035_label(&"a".repeat(64)), Err(too_long(63, None)));
}

#[test]
fn qualified_names() {
    let good = [
        "simple",
        "now-with-dashes",
        "1-starts-with-num",
        "1234",
        "simple/simple",
        "now-with-dashes/simple",
        "now-with-dashes/now-with-dashes",
        "now.with.dots/simple",
        "now-with.dashes-and.dots/simple",
        "1-num.2-num/3-num",
        "1234/5678",
        "1.2.3.4/5678",
        "Uppercase_Is_OK_123",
        "example.com/Uppercase_Is_OK_123",
        "requests.storage-foo",
    ];
    for value in good {
        assert_eq!(is_qualified_name(value), Ok(()), "{value}");
    }
    let long_name = format!("abc/{}", "a".repeat(63));
    let long_prefix = format!("{}/abc", "a".repeat(253));
    assert_eq!(is_qualified_name(&long_name), Ok(()));
    assert_eq!(is_qualified_name(&long_prefix), Ok(()));

    let name_format = ValidationError {
        part: Some(KeyPart::Name),
        kind: ValidationErrorKind::Format(Format::QualifiedName),
    };
    let prefix_format = ValidationError {
        part: Some(KeyPart::Prefix),
        kind: ValidationErrorKind::Format(Format::Dns1123Subdomain),
    };
    let name_empty = ValidationError {
        part: Some(KeyPart::Name),
        kind: ValidationErrorKind::Empty,
    };
    let prefix_empty = ValidationError {
        part: Some(KeyPart::Prefix),
        kind: ValidationErrorKind::Empty,
    };
    let slashes = ValidationError {
        part: None,
        kind: ValidationErrorKind::TooManySlashes,
    };
    let bad = [
        ("", name_empty),
        ("nospecialchars%^=@", name_format),
        ("cantendwithadash-", name_format),
        ("-cantstartwithadash-", name_format),
        ("only/one/slash", slashes),
        ("Example.com/abc", prefix_format),
        ("example_com/abc", prefix_format),
        ("example.com/", name_empty),
        ("/simple", prefix_empty),
        ("has space", name_format),
    ];
    for (value, expected) in bad {
        assert_eq!(is_qualified_name(value), Err(expected), "{value:?}");
    }
    let too_long_name = format!("abc/{}", "a".repeat(64));
    let too_long_prefix = format!("{}/abc", "a".repeat(254));
    assert_eq!(
        is_qualified_name(&too_long_name),
        Err(too_long(63, Some(KeyPart::Name)))
    );
    assert_eq!(
        is_qualified_name(&too_long_prefix),
        Err(too_long(253, Some(KeyPart::Prefix)))
    );
}

#[test]
fn label_values() {
    let good = [
        "simple",
        "now-with-dashes",
        "1-starts-with-num",
        "end-with-num-1",
        "1234",
        "",
        "Uppercase_Is_OK_123",
        "a.b_c-d",
    ];
    for value in good.iter().copied().chain([&*"a".repeat(63)]) {
        assert_eq!(is_valid_label_value(value), Ok(()), "{value}");
    }
    let bad = [
        "nospecialchars%^=@",
        "Tama-nui-te-rā.is_Māori.sun",
        "\\backslashes\\are\\bad",
        "-starts-with-dash",
        "ends-with-dash-",
        ".starts.with.dot",
        "ends.with.dot.",
    ];
    for value in bad {
        assert_eq!(
            is_valid_label_value(value),
            Err(format(Format::LabelValue)),
            "{value:?}"
        );
    }
    assert_eq!(
        is_valid_label_value(&"a".repeat(64)),
        Err(too_long(63, None))
    );
}

#[test]
fn messages() {
    let message = |result: Result<(), ValidationError>| result.unwrap_err().to_string();
    assert_eq!(
        message(is_dns1123_label(&"a".repeat(64))),
        "must be no more than 63 characters"
    );
    assert_eq!(
        message(is_qualified_name("/a")),
        "prefix part must be non-empty"
    );
    assert_eq!(
        message(is_qualified_name("a/-b")),
        "name part must consist of alphanumeric characters, '-', '_' or '.', and must \
         start and end with an alphanumeric character (e.g. 'MyName', 'my.name' or \
         '123-abc')"
    );
    assert!(message(is_dns1123_subdomain("A")).starts_with("a lowercase RFC 1123 subdomain"));
}
