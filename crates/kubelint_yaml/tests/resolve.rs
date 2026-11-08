use kubelint_yaml::ResolvedScalar::{self, Bool, Float, Int, Null, String as Str, Uint};
use kubelint_yaml::{Scalar, ScalarStyle};

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
        "yEs", "tRUE", "nul", "Y ", "maybe", "1.2.3", "v1", "e5", "0x", "-", "+", ".", "1e", "--1",
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
