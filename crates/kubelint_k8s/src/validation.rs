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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValidationErrorKind {
    Empty,
    /// The value is longer than `max` bytes.
    TooLong {
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
