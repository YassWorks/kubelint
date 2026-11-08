use super::{millis, suggest_suffix};

#[test]
fn suffix_suggestions() {
    let cases = [
        (
            "512",
            "mb",
            Some("write `512Mi` (powers of 1024) or `512M` (powers of 1000)"),
        ),
        (
            "1",
            "GB",
            Some("write `1Gi` (powers of 1024) or `1G` (powers of 1000)"),
        ),
        (
            "1",
            "K",
            Some("write `1Ki` (powers of 1024) or `1k` (powers of 1000)"),
        ),
        ("2", "gi", Some("suffixes are case-sensitive: write `2Gi`")),
        ("2", "MiB", Some("suffixes are case-sensitive: write `2Mi`")),
        ("100", "bytes", Some("bytes need no unit: write `100`")),
        (
            "0.5",
            "cores",
            Some("CPU is counted in cores without a unit: write `0.5` or `500m`"),
        ),
        (
            "2",
            "CPU",
            Some("CPU is counted in cores without a unit: write `2` or `2000m`"),
        ),
        (
            "250",
            "mcores",
            Some("millicores are written with `m`: write `250m`"),
        ),
        ("20", " Gi", Some("remove the space: write `20Gi`")),
        (
            "1",
            " CPU",
            Some("CPU is counted in cores without a unit: write `1` or `1000m`"),
        ),
        ("1", "x", None),
        ("1", "ib", None),
    ];
    for (number, suffix, expected) in cases {
        assert_eq!(
            suggest_suffix(number, suffix).as_deref(),
            expected,
            "{number}{suffix}"
        );
    }
}

#[test]
fn millis_of_numbers() {
    assert_eq!(millis("0.5").as_deref(), Some("500"));
    assert_eq!(millis(".25").as_deref(), Some("250"));
    assert_eq!(millis("1.").as_deref(), Some("1000"));
    assert_eq!(millis("0").as_deref(), Some("0"));
    assert_eq!(millis("0.0001"), None);
    assert_eq!(millis("-1"), None);
}
