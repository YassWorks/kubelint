//! Kubernetes-style scalar typing.
//!
//! Kubernetes decodes manifests with `sigs.k8s.io/yaml`, which reads YAML through
//! `gopkg.in/yaml.v2` and converts it to JSON. That decoder follows YAML 1.1 for plain
//! scalars: `yes`, `no`, `on`, `off`, `y` and `n` are booleans, `0777` is octal, and
//! `1_000` is an integer. The rules below mirror `resolve.go` in yaml.v2
//! (<https://github.com/go-yaml/yaml/blob/v2.4.0/resolve.go>).

use crate::{Scalar, ScalarStyle};

/// What Kubernetes' YAML decoder turns a scalar into.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ResolvedScalar {
    /// `~`, `null`, `Null`, `NULL` or an empty plain scalar.
    Null,
    /// `true`/`false` in the spellings yaml.v2 accepts, plus `yes`/`no`, `on`/`off`, `y`/`n`.
    Bool(bool),
    /// An integer that fits in `i64`.
    Int(i64),
    /// A non-negative integer that only fits in `u64`.
    Uint(u64),
    /// A float, including `.inf` and `.nan` spellings and integers too large for `u64`.
    Float(f64),
    /// Everything else, and every quoted or block scalar.
    String,
}

impl Scalar {
    /// Decides the type Kubernetes would give this scalar.
    ///
    /// Quoted and block scalars are always strings. Plain scalars follow yaml.v2.
    ///
    /// ```
    /// use kubelint_yaml::ResolvedScalar;
    ///
    /// let doc = &kubelint_yaml::parse("a: yes\nb: \"yes\"\nc: 0x1f").documents[0];
    /// let root = doc.root.as_ref().unwrap();
    /// let resolve = |k| root.get(k).unwrap().as_scalar().unwrap().resolve_kubernetes();
    /// assert_eq!(resolve("a"), ResolvedScalar::Bool(true));
    /// assert_eq!(resolve("b"), ResolvedScalar::String);
    /// assert_eq!(resolve("c"), ResolvedScalar::Int(31));
    /// ```
    pub fn resolve_kubernetes(&self) -> ResolvedScalar {
        match self.style {
            ScalarStyle::Plain => resolve_plain(&self.value),
            _ => ResolvedScalar::String,
        }
    }
}

fn resolve_plain(s: &str) -> ResolvedScalar {
    match s {
        "" | "~" | "null" | "Null" | "NULL" => return ResolvedScalar::Null,
        "y" | "Y" | "yes" | "Yes" | "YES" | "on" | "On" | "ON" | "true" | "True" | "TRUE" => {
            return ResolvedScalar::Bool(true);
        }
        "n" | "N" | "no" | "No" | "NO" | "off" | "Off" | "OFF" | "false" | "False" | "FALSE" => {
            return ResolvedScalar::Bool(false);
        }
        ".inf" | ".Inf" | ".INF" | "+.inf" | "+.Inf" | "+.INF" => {
            return ResolvedScalar::Float(f64::INFINITY);
        }
        "-.inf" | "-.Inf" | "-.INF" => return ResolvedScalar::Float(f64::NEG_INFINITY),
        ".nan" | ".NaN" | ".NAN" => return ResolvedScalar::Float(f64::NAN),
        _ => {}
    }
    // yaml.v2 only tries numbers when the first character can start one.
    let first = s.as_bytes()[0];
    if !(first.is_ascii_digit() || matches!(first, b'+' | b'-' | b'.')) {
        return ResolvedScalar::String;
    }
    let plain = s.replace('_', "");
    if let Some(i) = parse_int(&plain) {
        return ResolvedScalar::Int(i);
    }
    // `ParseUint` in Go does not accept a sign.
    if !plain.starts_with(['+', '-'])
        && let Some(u) = parse_uint(&plain)
    {
        return ResolvedScalar::Uint(u);
    }
    if is_float_syntax(&plain)
        && let Ok(f) = plain.parse::<f64>()
    {
        return ResolvedScalar::Float(f);
    }
    ResolvedScalar::String
}

/// Splits Go-style base prefixes (`0x`, `0o`, `0b`, leading `0` for octal).
fn split_base(digits: &str) -> (u32, &str) {
    match digits.as_bytes() {
        [b'0', b'x' | b'X', _, ..] => (16, &digits[2..]),
        [b'0', b'o' | b'O', _, ..] => (8, &digits[2..]),
        [b'0', b'b' | b'B', _, ..] => (2, &digits[2..]),
        [b'0', _, ..] => (8, &digits[1..]),
        _ => (10, digits),
    }
}

fn parse_int(s: &str) -> Option<i64> {
    let (negative, unsigned) = match s.as_bytes().first()? {
        b'-' => (true, &s[1..]),
        b'+' => (false, &s[1..]),
        _ => (false, s),
    };
    let (radix, digits) = split_base(unsigned);
    if digits.is_empty() || digits.starts_with(['+', '-']) {
        return None;
    }
    // Parse with the sign attached so that `i64::MIN` works.
    let signed = if negative {
        format!("-{digits}")
    } else {
        digits.to_owned()
    };
    i64::from_str_radix(&signed, radix).ok()
}

fn parse_uint(s: &str) -> Option<u64> {
    let (radix, digits) = split_base(s);
    if digits.is_empty() || digits.starts_with('+') {
        return None;
    }
    u64::from_str_radix(digits, radix).ok()
}

/// `^[-+]?(\.[0-9]+|[0-9]+(\.[0-9]*)?)([eE][-+]?[0-9]+)?$`
fn is_float_syntax(s: &str) -> bool {
    let s = s.strip_prefix(['+', '-']).unwrap_or(s);
    let (mantissa, exponent) = match s.find(['e', 'E']) {
        Some(i) => (&s[..i], Some(&s[i + 1..])),
        None => (s, None),
    };
    let digits = |t: &str| !t.is_empty() && t.bytes().all(|b| b.is_ascii_digit());
    let mantissa_ok = match mantissa.split_once('.') {
        Some(("", frac)) => digits(frac),
        Some((int, frac)) => digits(int) && (frac.is_empty() || digits(frac)),
        None => digits(mantissa),
    };
    let exponent_ok = exponent.is_none_or(|e| digits(e.strip_prefix(['+', '-']).unwrap_or(e)));
    mantissa_ok && exponent_ok
}
