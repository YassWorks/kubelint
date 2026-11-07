//! Kubernetes' syntax checks for names, label keys and label values.
//!
//! These mirror `staging/src/k8s.io/apimachinery/pkg/util/validation/validation.go`
//! (`IsDNS1123Label`, `IsDNS1123Subdomain`, `IsDNS1035Label`, `IsQualifiedName`,
//! `IsValidLabelValue`), with the same limits, the same order of checks and the same
//! wording, minus the regular expression Kubernetes appends to its messages.
//!
//! Kubernetes can return several messages for one value (too long *and* badly formed).
//! These functions return the first one, which is the one Kubernetes lists first.

use std::fmt;

/// The longest DNS label (RFC 1123 or RFC 1035), and the longest label value or label
/// key name part.
pub const MAX_LABEL_LENGTH: usize = 63;

/// The longest DNS subdomain (RFC 1123), and the longest label key prefix.
pub const MAX_SUBDOMAIN_LENGTH: usize = 253;

/// Why a name, label key or label value is invalid.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ValidationError {
    /// For a label key with a `prefix/`: which side of the `/` is wrong. `None` for
    /// everything else.
    pub part: Option<KeyPart>,
    /// What is wrong.
    pub kind: ValidationErrorKind,
}

/// The two sides of a qualified name such as `app.kubernetes.io/name`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyPart {
    /// The DNS subdomain before the `/`, `app.kubernetes.io`.
    Prefix,
    /// The part after the `/` (or the whole key without one), `name`.
    Name,
}

/// What is wrong in a [`ValidationError`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValidationErrorKind {
    /// The value is empty where it may not be.
    Empty,
    /// The value is longer than `max` bytes.
    TooLong {
        /// The limit.
        max: usize,
    },
    /// The value has characters that are not allowed, or starts or ends with one that
    /// may only appear inside.
    Format(Format),
    /// A qualified name with more than one `/`.
    TooManySlashes,
}

/// The syntax a value failed, for [`ValidationErrorKind::Format`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Format {
    /// An RFC 1123 label: `my-name`, `123-abc`.
    Dns1123Label,
    /// An RFC 1123 subdomain: `example.com`.
    Dns1123Subdomain,
    /// An RFC 1035 label, which must start with a letter: `my-name`, `abc-123`.
    Dns1035Label,
    /// The name part of a qualified name: `MyName`, `my.name`, `123-abc`.
    QualifiedName,
    /// A label value: empty, `MyValue`, `my_value`, `12345`.
    LabelValue,
}

impl fmt::Display for ValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.part {
            Some(KeyPart::Prefix) => f.write_str("prefix part ")?,
            Some(KeyPart::Name) => f.write_str("name part ")?,
            None => {}
        }
        match self.kind {
            ValidationErrorKind::Empty => f.write_str("must be non-empty"),
            ValidationErrorKind::TooLong { max } => {
                write!(f, "must be no more than {max} characters")
            }
            ValidationErrorKind::Format(format) => f.write_str(format.message()),
            ValidationErrorKind::TooManySlashes => f.write_str(
                "a qualified name must consist of a name with an optional DNS subdomain \
                 prefix and '/' (e.g. 'example.com/MyName')",
            ),
        }
    }
}

impl std::error::Error for ValidationError {}

impl Format {
    /// Kubernetes' explanation of the syntax, with its examples.
    pub fn message(self) -> &'static str {
        match self {
            Self::Dns1123Label => {
                "a lowercase RFC 1123 label must consist of lower case alphanumeric \
                 characters or '-', and must start and end with an alphanumeric character \
                 (e.g. 'my-name' or '123-abc')"
            }
            Self::Dns1123Subdomain => {
                "a lowercase RFC 1123 subdomain must consist of lower case alphanumeric \
                 characters, '-' or '.', and must start and end with an alphanumeric \
                 character (e.g. 'example.com')"
            }
            Self::Dns1035Label => {
                "a DNS-1035 label must consist of lower case alphanumeric characters or \
                 '-', start with an alphabetic character, and end with an alphanumeric \
                 character (e.g. 'my-name' or 'abc-123')"
            }
            Self::QualifiedName => {
                "must consist of alphanumeric characters, '-', '_' or '.', and must start \
                 and end with an alphanumeric character (e.g. 'MyName', 'my.name' or \
                 '123-abc')"
            }
            Self::LabelValue => {
                "a valid label must be an empty string or consist of alphanumeric \
                 characters, '-', '_' or '.', and must start and end with an alphanumeric \
                 character (e.g. 'MyValue', 'my_value' or '12345')"
            }
        }
    }
}

/// Checks an RFC 1123 label, the format of namespace names and many other names:
/// at most 63 lowercase letters, digits and `-`, starting and ending with a letter or
/// digit.
///
/// ```
/// use kubelint_k8s::is_dns1123_label;
///
/// assert!(is_dns1123_label("web-1").is_ok());
/// assert!(is_dns1123_label("Web").is_err());
/// assert!(is_dns1123_label("a.b").is_err());
/// ```
pub fn is_dns1123_label(value: &str) -> Result<(), ValidationError> {
    check_length(value, MAX_LABEL_LENGTH, None)?;
    check_format(
        is_dns_label_syntax(value, u8::is_ascii_alphanumeric),
        Format::Dns1123Label,
    )
}

/// Checks an RFC 1123 subdomain, the format of most object names: at most 253
/// characters of dot-separated RFC 1123 labels.
///
/// ```
/// use kubelint_k8s::is_dns1123_subdomain;
///
/// assert!(is_dns1123_subdomain("my-app.v2").is_ok());
/// assert!(is_dns1123_subdomain("my_app").is_err());
/// assert!(is_dns1123_subdomain("").is_err());
/// ```
pub fn is_dns1123_subdomain(value: &str) -> Result<(), ValidationError> {
    is_subdomain(value, None)
}

/// Checks an RFC 1035 label, the format of Service names: like an RFC 1123 label, but
/// the first character must be a letter.
///
/// ```
/// use kubelint_k8s::is_dns1035_label;
///
/// assert!(is_dns1035_label("web").is_ok());
/// assert!(is_dns1035_label("1web").is_err());
/// ```
pub fn is_dns1035_label(value: &str) -> Result<(), ValidationError> {
    check_length(value, MAX_LABEL_LENGTH, None)?;
    check_format(
        is_dns_label_syntax(value, u8::is_ascii_lowercase),
        Format::Dns1035Label,
    )
}

/// Checks a qualified name, the format of label and annotation keys: an optional DNS
/// subdomain prefix and `/`, then a name of at most 63 letters, digits, `-`, `_` and
/// `.`, starting and ending with a letter or digit.
///
/// ```
/// use kubelint_k8s::{KeyPart, is_qualified_name};
///
/// assert!(is_qualified_name("app.kubernetes.io/name").is_ok());
/// assert!(is_qualified_name("Tier").is_ok());
/// let error = is_qualified_name("Example.com/tier").unwrap_err();
/// assert_eq!(error.part, Some(KeyPart::Prefix));
/// ```
pub fn is_qualified_name(value: &str) -> Result<(), ValidationError> {
    let name = match value.split('/').collect::<Vec<_>>()[..] {
        [name] => name,
        [prefix, name] => {
            if prefix.is_empty() {
                return Err(ValidationError {
                    part: Some(KeyPart::Prefix),
                    kind: ValidationErrorKind::Empty,
                });
            }
            is_subdomain(prefix, Some(KeyPart::Prefix))?;
            name
        }
        _ => {
            return Err(ValidationError {
                part: None,
                kind: ValidationErrorKind::TooManySlashes,
            });
        }
    };
    let part = Some(KeyPart::Name);
    if name.is_empty() {
        return Err(ValidationError {
            part,
            kind: ValidationErrorKind::Empty,
        });
    }
    check_length(name, MAX_LABEL_LENGTH, part)?;
    if is_qualified_syntax(name) {
        Ok(())
    } else {
        Err(ValidationError {
            part,
            kind: ValidationErrorKind::Format(Format::QualifiedName),
        })
    }
}

/// Checks a label value: empty, or at most 63 letters, digits, `-`, `_` and `.`,
/// starting and ending with a letter or digit.
///
/// ```
/// use kubelint_k8s::is_valid_label_value;
///
/// assert!(is_valid_label_value("").is_ok());
/// assert!(is_valid_label_value("v1.2_beta").is_ok());
/// assert!(is_valid_label_value("hello world").is_err());
/// ```
pub fn is_valid_label_value(value: &str) -> Result<(), ValidationError> {
    check_length(value, MAX_LABEL_LENGTH, None)?;
    check_format(
        value.is_empty() || is_qualified_syntax(value),
        Format::LabelValue,
    )
}

fn is_subdomain(value: &str, part: Option<KeyPart>) -> Result<(), ValidationError> {
    check_length(value, MAX_SUBDOMAIN_LENGTH, part)?;
    // `label(\.label)*`: every dot-separated piece is an RFC 1123 label.
    let valid = value
        .split('.')
        .all(|label| is_dns_label_syntax(label, u8::is_ascii_alphanumeric));
    if valid {
        Ok(())
    } else {
        Err(ValidationError {
            part,
            kind: ValidationErrorKind::Format(Format::Dns1123Subdomain),
        })
    }
}

fn check_length(value: &str, max: usize, part: Option<KeyPart>) -> Result<(), ValidationError> {
    if value.len() > max {
        Err(ValidationError {
            part,
            kind: ValidationErrorKind::TooLong { max },
        })
    } else {
        Ok(())
    }
}

fn check_format(valid: bool, format: Format) -> Result<(), ValidationError> {
    if valid {
        Ok(())
    } else {
        Err(ValidationError {
            part: None,
            kind: ValidationErrorKind::Format(format),
        })
    }
}

/// `[a-z0-9-]+`, first byte accepted by `first`, last byte a lowercase letter or digit.
fn is_dns_label_syntax(value: &str, first: fn(&u8) -> bool) -> bool {
    let bytes = value.as_bytes();
    let lower_alnum = |b: &u8| b.is_ascii_lowercase() || b.is_ascii_digit();
    match (bytes.first(), bytes.last()) {
        (Some(head), Some(tail)) => {
            first(head)
                && lower_alnum(head)
                && lower_alnum(tail)
                && bytes.iter().all(|b| lower_alnum(b) || *b == b'-')
        }
        _ => false,
    }
}

/// `([A-Za-z0-9][-A-Za-z0-9_.]*)?[A-Za-z0-9]`.
fn is_qualified_syntax(value: &str) -> bool {
    let bytes = value.as_bytes();
    match (bytes.first(), bytes.last()) {
        (Some(head), Some(tail)) => {
            head.is_ascii_alphanumeric()
                && tail.is_ascii_alphanumeric()
                && bytes
                    .iter()
                    .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.'))
        }
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::{
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
        "", "A", "ABC", "aBc", "A1", "A-1", "1-A", "-", "a-", "-a", "1-", "-1", "_", "a_", "_a",
        "a_b", "1_", "_1", "1_2", ".", "a.", ".a", "a.b", "1.", ".1", "1.2", " ", "a ", " a",
        "a b", "1 ", " 1", "1 2",
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
}
