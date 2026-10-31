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
    let lower = digits.as_bytes();
    if lower.len() > 2 && lower[0] == b'0' {
        match lower[1] {
            b'x' | b'X' => return (16, &digits[2..]),
            b'o' | b'O' => return (8, &digits[2..]),
            b'b' | b'B' => return (2, &digits[2..]),
            _ => {}
        }
    }
    if lower.len() > 1 && lower[0] == b'0' {
        return (8, &digits[1..]);
    }
    (10, digits)
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

#[cfg(test)]
mod tests {
    use super::ResolvedScalar::{self, Bool, Float, Int, Null, String as Str, Uint};
    use crate::{Scalar, ScalarStyle};

    fn plain(value: &str) -> ResolvedScalar {
        Scalar {
            value: value.to_owned(),
            style: ScalarStyle::Plain,
        }
        .resolve_kubernetes()
    }

    #[test]
    fn nulls() {
        for s in ["", "~", "null", "Null", "NULL"] {
            assert_eq!(plain(s), Null, "{s:?}");
        }
    }

    #[test]
    fn yaml_1_1_booleans() {
        for s in [
            "y", "Y", "yes", "Yes", "YES", "on", "On", "ON", "true", "True", "TRUE",
        ] {
            assert_eq!(plain(s), Bool(true), "{s:?}");
        }
        for s in [
            "n", "N", "no", "No", "NO", "off", "Off", "OFF", "false", "False", "FALSE",
        ] {
            assert_eq!(plain(s), Bool(false), "{s:?}");
        }
    }

    #[test]
    fn near_misses_are_strings() {
        for s in [
            "yEs", "tRUE", "nul", "Y ", "maybe", "1.2.3", "v1", "e5", "0x", "-", "+", ".", "1e",
            "--1",
        ] {
            assert_eq!(plain(s), Str, "{s:?}");
        }
    }

    #[test]
    fn integers() {
        let cases = [
            ("0", 0),
            ("8080", 8080),
            ("-12", -12),
            ("+7", 7),
            ("1_000", 1000),
            ("0x1f", 31),
            ("0X1F", 31),
            ("0o17", 15),
            ("0b101", 5),
            ("0777", 511),
            ("-0x10", -16),
            ("-9223372036854775808", i64::MIN),
            ("9223372036854775807", i64::MAX),
        ];
        for (s, want) in cases {
            assert_eq!(plain(s), Int(want), "{s:?}");
        }
    }

    #[test]
    fn large_integers() {
        assert_eq!(
            plain("9223372036854775808"),
            Uint(9_223_372_036_854_775_808)
        );
        assert_eq!(plain("18446744073709551615"), Uint(u64::MAX));
        assert_eq!(
            plain("18446744073709551616"),
            Float(18_446_744_073_709_551_616.0)
        );
    }

    #[test]
    fn floats() {
        let cases = [
            ("1.5", 1.5),
            (".5", 0.5),
            ("5.", 5.0),
            ("-1e3", -1000.0),
            ("1E-2", 0.01),
            ("089", 89.0),
        ];
        for (s, want) in cases {
            assert_eq!(plain(s), Float(want), "{s:?}");
        }
        assert_eq!(plain(".inf"), Float(f64::INFINITY));
        assert_eq!(plain("-.INF"), Float(f64::NEG_INFINITY));
        assert!(matches!(plain(".NaN"), Float(f) if f.is_nan()));
    }

    #[test]
    fn quoted_and_block_scalars_are_strings() {
        for style in [
            ScalarStyle::SingleQuoted,
            ScalarStyle::DoubleQuoted,
            ScalarStyle::Literal,
            ScalarStyle::Folded,
        ] {
            let s = Scalar {
                value: "yes".to_owned(),
                style,
            };
            assert_eq!(s.resolve_kubernetes(), Str, "{style:?}");
        }
    }
}
