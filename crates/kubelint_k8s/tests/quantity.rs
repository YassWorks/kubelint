//! Table-driven tests of the resource quantity parser against Kubernetes' grammar.

use kubelint_k8s::{Quantity, QuantityError, Suffix};

#[test]
fn valid_quantities() {
    let cases: &[(&str, &str, Suffix, f64)] = &[
        // Plain numbers in every shape the grammar allows.
        ("0", "0", Suffix::Decimal(0), 0.0),
        ("1", "1", Suffix::Decimal(0), 1.0),
        ("007", "007", Suffix::Decimal(0), 7.0),
        ("1.5", "1.5", Suffix::Decimal(0), 1.5),
        ("1.", "1.", Suffix::Decimal(0), 1.0),
        (".5", ".5", Suffix::Decimal(0), 0.5),
        ("+2", "+2", Suffix::Decimal(0), 2.0),
        ("-2", "-2", Suffix::Decimal(0), -2.0),
        ("-.5", "-.5", Suffix::Decimal(0), -0.5),
        ("+.5", "+.5", Suffix::Decimal(0), 0.5),
        // Decimal SI suffixes.
        ("100n", "100", Suffix::Decimal(-9), 100e-9),
        ("100u", "100", Suffix::Decimal(-6), 100e-6),
        ("500m", "500", Suffix::Decimal(-3), 0.5),
        ("2k", "2", Suffix::Decimal(3), 2e3),
        ("128M", "128", Suffix::Decimal(6), 128e6),
        ("1G", "1", Suffix::Decimal(9), 1e9),
        ("1T", "1", Suffix::Decimal(12), 1e12),
        ("1P", "1", Suffix::Decimal(15), 1e15),
        ("1E", "1", Suffix::Decimal(18), 1e18),
        ("1.5G", "1.5", Suffix::Decimal(9), 1.5e9),
        // Binary SI suffixes.
        ("1Ki", "1", Suffix::Binary(1), 1024.0),
        ("128Mi", "128", Suffix::Binary(2), 128.0 * 1024.0 * 1024.0),
        ("1Gi", "1", Suffix::Binary(3), 1024_f64.powi(3)),
        ("1Ti", "1", Suffix::Binary(4), 1024_f64.powi(4)),
        ("1Pi", "1", Suffix::Binary(5), 1024_f64.powi(5)),
        ("1Ei", "1", Suffix::Binary(6), 1024_f64.powi(6)),
        ("1.5Gi", "1.5", Suffix::Binary(3), 1.5 * 1024_f64.powi(3)),
        ("0Mi", "0", Suffix::Binary(2), 0.0),
        // Decimal exponents.
        ("1e3", "1", Suffix::Exponent(3), 1e3),
        ("1E3", "1", Suffix::Exponent(3), 1e3),
        ("1e+3", "1", Suffix::Exponent(3), 1e3),
        ("15e-1", "15", Suffix::Exponent(-1), 1.5),
        ("1.5e2", "1.5", Suffix::Exponent(2), 150.0),
        ("1e0", "1", Suffix::Exponent(0), 1.0),
    ];
    // Kubernetes reads the exponent as an int64, so this is accepted (and absurd).
    let huge = Quantity::parse("1e99999999999").map(|q| q.suffix);
    assert_eq!(huge, Ok(Suffix::Exponent(i32::MAX)));
    for &(text, number, suffix, value) in cases {
        let quantity = Quantity::parse(text).unwrap_or_else(|err| panic!("{text}: {err}"));
        assert_eq!(
            (quantity.number, quantity.suffix),
            (number, suffix),
            "{text}"
        );
        let difference = (quantity.as_f64() - value).abs();
        assert!(
            difference <= value.abs() * 1e-12,
            "{text}: {}",
            quantity.as_f64()
        );
    }
}

#[test]
fn invalid_quantities() {
    let cases: &[(&str, QuantityError<'_>)] = &[
        ("", QuantityError::Empty),
        // No digits in the number.
        (".", QuantityError::InvalidNumber),
        ("-", QuantityError::InvalidNumber),
        ("+", QuantityError::InvalidNumber),
        ("Mi", QuantityError::InvalidNumber),
        ("m", QuantityError::InvalidNumber),
        ("abc", QuantityError::InvalidNumber),
        ("-.Gi", QuantityError::InvalidNumber),
        // Whitespace is never allowed.
        (" 1", QuantityError::InvalidNumber),
        ("1 ", QuantityError::InvalidSuffix(" ")),
        ("1 Gi", QuantityError::InvalidSuffix(" Gi")),
        // Units people write that Kubernetes does not know.
        ("512mb", QuantityError::InvalidSuffix("mb")),
        ("512MB", QuantityError::InvalidSuffix("MB")),
        ("1GB", QuantityError::InvalidSuffix("GB")),
        ("1gi", QuantityError::InvalidSuffix("gi")),
        ("1mi", QuantityError::InvalidSuffix("mi")),
        ("1K", QuantityError::InvalidSuffix("K")),
        ("1Kb", QuantityError::InvalidSuffix("Kb")),
        ("0.5cores", QuantityError::InvalidSuffix("cores")),
        ("100%", QuantityError::InvalidSuffix("%")),
        ("1Mii", QuantityError::InvalidSuffix("Mii")),
        ("1i", QuantityError::InvalidSuffix("i")),
        // Malformed numbers and exponents.
        ("1.5.5", QuantityError::InvalidSuffix(".5")),
        ("1..5", QuantityError::InvalidSuffix(".5")),
        ("--1", QuantityError::InvalidNumber),
        ("+-1", QuantityError::InvalidNumber),
        ("1e", QuantityError::InvalidSuffix("e")),
        ("1e+", QuantityError::InvalidSuffix("e+")),
        ("1e1.5", QuantityError::InvalidSuffix("e1.5")),
        ("1e++1", QuantityError::InvalidSuffix("e++1")),
        ("1e3Mi", QuantityError::InvalidSuffix("e3Mi")),
        ("1Ee3", QuantityError::InvalidSuffix("Ee3")),
        (
            "1e99999999999999999999",
            QuantityError::InvalidSuffix("e99999999999999999999"),
        ),
        ("1,5", QuantityError::InvalidSuffix(",5")),
        ("１", QuantityError::InvalidNumber),
    ];
    for &(text, expected) in cases {
        assert_eq!(Quantity::parse(text), Err(expected), "{text:?}");
    }
}

#[test]
fn errors_explain_themselves() {
    assert_eq!(
        QuantityError::InvalidSuffix("mb").to_string(),
        "`mb` is not a quantity suffix"
    );
    assert_eq!(QuantityError::Empty.to_string(), "the quantity is empty");
}
