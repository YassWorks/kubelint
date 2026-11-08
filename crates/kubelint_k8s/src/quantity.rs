use std::fmt;

/// A parsed resource quantity: a signed decimal number and a suffix.
///
/// The grammar is the one documented on Kubernetes' `resource.Quantity`:
///
/// ```text
/// <quantity>        ::= <signedNumber><suffix>
/// <signedNumber>    ::= <number> | +<number> | -<number>
/// <number>          ::= <digits> | <digits>.<digits> | <digits>. | .<digits>
/// <suffix>          ::= <binarySI> | <decimalExponent> | <decimalSI>
/// <binarySI>        ::= Ki | Mi | Gi | Ti | Pi | Ei
/// <decimalSI>       ::= n | u | m | "" | k | M | G | T | P | E
/// <decimalExponent> ::= e <signedInteger> | E <signedInteger>
/// ```
///
/// Source: `staging/src/k8s.io/apimachinery/pkg/api/resource/quantity.go`
/// (`ParseQuantity`, `parseQuantityString`) and `suffix.go`. Suffixes are
/// case-sensitive: `1K` and `1mi` are invalid. No whitespace is allowed anywhere.
///
/// Unlike Kubernetes, which reads some of them as zero, a number without any digit
/// (`.`, `-`, `+Mi`) is rejected.
///
/// ```
/// use kubelint_k8s::{Quantity, Suffix};
///
/// let q = Quantity::parse("128Mi").unwrap();
/// assert_eq!(q.number, "128");
/// assert_eq!(q.suffix, Suffix::Binary(2));
/// assert_eq!(q.as_f64(), 128.0 * 1024.0 * 1024.0);
/// assert_eq!(Quantity::parse("500m").unwrap().as_f64(), 0.5);
/// assert!(Quantity::parse("512mb").is_err());
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Quantity<'a> {
    /// The number as written, with its sign: `128`, `-1.5`, `.5`.
    pub number: &'a str,
    pub suffix: Suffix,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Suffix {
    /// A power of 1024: `Ki` is 1, `Mi` 2, up to `Ei` 6.
    Binary(u8),
    /// A decimal SI suffix, as a power of 10: `n` is -9, `m` -3, none 0, `k` 3, `M` 6,
    /// up to `E` 18.
    Decimal(i8),
    /// A decimal exponent such as `e3` or `E-2`, as the power of 10.
    Exponent(i32),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuantityError<'a> {
    Empty,
    /// The string does not start with a number (an optional sign, then digits with at
    /// most one `.`).
    InvalidNumber,
    /// The number is followed by something that is not a known suffix, such as `mb`,
    /// `cores` or `K`. Holds that text.
    InvalidSuffix(&'a str),
}

impl fmt::Display for QuantityError<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => f.write_str("the quantity is empty"),
            Self::InvalidNumber => f.write_str("the quantity does not start with a number"),
            Self::InvalidSuffix(suffix) => write!(f, "`{suffix}` is not a quantity suffix"),
        }
    }
}

impl std::error::Error for QuantityError<'_> {}

impl<'a> Quantity<'a> {
    /// Parses `text` with Kubernetes' quantity grammar (see [`Quantity`]).
    ///
    /// ```
    /// use kubelint_k8s::{Quantity, QuantityError};
    ///
    /// assert_eq!(Quantity::parse(""), Err(QuantityError::Empty));
    /// assert_eq!(Quantity::parse("Mi"), Err(QuantityError::InvalidNumber));
    /// assert_eq!(Quantity::parse("0.5cores"), Err(QuantityError::InvalidSuffix("cores")));
    /// ```
    pub fn parse(text: &'a str) -> Result<Self, QuantityError<'a>> {
        if text.is_empty() {
            return Err(QuantityError::Empty);
        }
        let bytes = text.as_bytes();
        let mut end = usize::from(matches!(bytes[0], b'+' | b'-'));
        let integer = count_digits(&bytes[end..]);
        end += integer;
        let mut fraction = 0;
        if bytes.get(end) == Some(&b'.') {
            fraction = count_digits(&bytes[end + 1..]);
            end += 1 + fraction;
        }
        if integer + fraction == 0 {
            return Err(QuantityError::InvalidNumber);
        }
        let (number, suffix) = text.split_at(end);
        let suffix = parse_suffix(suffix).ok_or(QuantityError::InvalidSuffix(suffix))?;
        Ok(Self { number, suffix })
    }

    /// The value as a floating-point number, for comparisons and messages. Very large
    /// or very precise quantities lose precision.
    ///
    /// ```
    /// use kubelint_k8s::Quantity;
    ///
    /// assert_eq!(Quantity::parse("1Ki").unwrap().as_f64(), 1024.0);
    /// assert_eq!(Quantity::parse("2k").unwrap().as_f64(), 2000.0);
    /// assert_eq!(Quantity::parse("-1.5e2").unwrap().as_f64(), -150.0);
    /// ```
    pub fn as_f64(&self) -> f64 {
        // `parse` only accepts numbers Rust's float parser also accepts (`1.`, `.5`,
        // `+1`), so this cannot fail.
        let number: f64 = self.number.parse().unwrap_or(0.0);
        let factor = match self.suffix {
            Suffix::Binary(power) => 1024_f64.powi(i32::from(power)),
            Suffix::Decimal(power) => 10_f64.powi(i32::from(power)),
            Suffix::Exponent(power) => 10_f64.powi(power),
        };
        number * factor
    }
}

fn count_digits(bytes: &[u8]) -> usize {
    bytes.iter().take_while(|b| b.is_ascii_digit()).count()
}

fn parse_suffix(suffix: &str) -> Option<Suffix> {
    let parsed = match suffix {
        "Ki" => Suffix::Binary(1),
        "Mi" => Suffix::Binary(2),
        "Gi" => Suffix::Binary(3),
        "Ti" => Suffix::Binary(4),
        "Pi" => Suffix::Binary(5),
        "Ei" => Suffix::Binary(6),
        "n" => Suffix::Decimal(-9),
        "u" => Suffix::Decimal(-6),
        "m" => Suffix::Decimal(-3),
        "" => Suffix::Decimal(0),
        "k" => Suffix::Decimal(3),
        "M" => Suffix::Decimal(6),
        "G" => Suffix::Decimal(9),
        "T" => Suffix::Decimal(12),
        "P" => Suffix::Decimal(15),
        "E" => Suffix::Decimal(18),
        _ => {
            let exponent = suffix.strip_prefix(['e', 'E'])?;
            // Only an optional sign and digits, as Kubernetes allows.
            let digits = exponent.strip_prefix(['+', '-']).unwrap_or(exponent);
            if digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_digit()) {
                return None;
            }
            // Kubernetes parses the exponent as an `int64`; anything that fits is
            // accepted. Clamp it, since such a value is absurd anyway.
            let exponent: i64 = exponent.parse().ok()?;
            Suffix::Exponent(i32::try_from(exponent).unwrap_or(if exponent < 0 {
                i32::MIN
            } else {
                i32::MAX
            }))
        }
    };
    Some(parsed)
}
