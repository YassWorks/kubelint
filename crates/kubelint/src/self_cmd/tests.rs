use std::fs;
use std::path::{MAIN_SEPARATOR, PathBuf};

use super::{Receipt, Version, manifest_version};

const LINUX_RECEIPT: &str = include_str!("../../tests/fixtures/receipts/linux.json");
const WINDOWS_RECEIPT: &str = include_str!("../../tests/fixtures/receipts/windows.json");

fn version(text: &str) -> Version {
    Version::parse(text).expect("a release version")
}

fn flat(dir: &str) -> Receipt {
    Receipt {
        prefix: dir.into(),
        bin_dir: dir.into(),
    }
}

#[test]
fn release_versions_parse() {
    assert_eq!(Version::parse("0.20.3"), Some(Version([0, 20, 3])));
    assert_eq!(Version::parse("12.0.105"), Some(Version([12, 0, 105])));
}

#[test]
fn other_text_is_not_a_release_version() {
    for text in [
        "",
        "1",
        "1.2",
        "1.2.3.4",
        "1.2.",
        ".1.2",
        "1..3",
        "v1.2.3",
        "1.2.3-rc.1",
        "1.2.x",
        " 1.2.3",
        "1.2.3\n",
        "-1.2.3",
        "18446744073709551616.0.0",
    ] {
        assert_eq!(Version::parse(text), None, "{text:?}");
    }
}

#[test]
fn versions_compare_number_by_number() {
    assert!(version("0.20.10") > version("0.20.9"));
    assert!(version("0.21.0") > version("0.20.99"));
    assert!(version("1.0.0") > version("0.99.99"));
    assert_eq!(version("0.20.3"), version("0.20.3"));
}

#[test]
fn a_version_prints_without_the_tag_prefix() {
    assert_eq!(version("0.20.10").to_string(), "0.20.10");
}

#[test]
fn the_manifest_names_the_version_of_its_tag() {
    let manifest = br#"{"dist_version":"0.34.0","announcement_tag":"v0.21.0","releases":[]}"#;
    assert_eq!(manifest_version(manifest), Some(version("0.21.0")));
}

#[test]
fn a_manifest_without_a_release_tag_names_no_version() {
    for manifest in [
        "",
        "not json",
        "[]",
        "{}",
        r#"{"announcement_tag":null}"#,
        r#"{"announcement_tag":"0.21.0"}"#,
        r#"{"announcement_tag":"v0.21"}"#,
        r#"{"announcement_tag":"kubelint-v0.21.0"}"#,
    ] {
        assert_eq!(manifest_version(manifest.as_bytes()), None, "{manifest}");
    }
}

#[test]
fn the_shell_installer_receipt_is_read() {
    assert_eq!(
        Receipt::parse(LINUX_RECEIPT),
        Some(flat("/root/.local/bin"))
    );
}

#[test]
fn the_powershell_installer_receipt_is_read() {
    let crlf = WINDOWS_RECEIPT.replace('\n', "\r\n");
    assert_eq!(
        Receipt::parse(&crlf),
        Some(flat(r"C:\Users\alice\.local\bin"))
    );
}

#[test]
fn a_cargo_home_receipt_has_its_binary_in_bin() {
    let text = r#"{"install_prefix":"/home/alice/.cargo","install_layout":"cargo-home"}"#;
    let parsed = Receipt::parse(text).expect("a receipt");
    assert_eq!(parsed.prefix, PathBuf::from("/home/alice/.cargo"));
    assert_eq!(
        parsed.bin_dir,
        PathBuf::from("/home/alice/.cargo").join("bin")
    );
}

#[test]
fn every_other_layout_has_its_binary_in_the_folder_itself() {
    for layout in [
        "",
        r#","install_layout":"flat""#,
        r#","install_layout":"hierarchical""#,
    ] {
        let text = format!(r#"{{"install_prefix":"/opt/kubelint"{layout}}}"#);
        assert_eq!(Receipt::parse(&text), Some(flat("/opt/kubelint")), "{text}");
    }
}

#[test]
fn text_that_is_not_a_receipt_is_rejected() {
    for text in [
        "",
        "not json",
        "[]",
        "{}",
        r#"{"install_prefix":42}"#,
        r#"{"install_prefix":null}"#,
        r#"{"install_prefix":"/root/.local/bin""#,
    ] {
        assert_eq!(Receipt::parse(text), None, "{text}");
    }
}

#[test]
fn a_receipt_is_for_the_binaries_in_its_folder_only() {
    let root = std::env::temp_dir().join("kubelint-self-update-receipt");
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(root.join("bin").join("nested")).expect("mkdir");
    for file in ["kubelint", "bin/kubelint", "bin/nested/kubelint"] {
        fs::write(root.join(file), "").expect("write");
    }
    let canonical = |file: &str| fs::canonicalize(root.join(file)).expect("exists");
    let bin = root.join("bin").display().to_string();
    let with_separator = format!("{bin}{MAIN_SEPARATOR}");

    assert!(flat(&bin).is_for(&canonical("bin/kubelint")));
    assert!(flat(&with_separator).is_for(&canonical("bin/kubelint")));
    assert!(!flat(&bin).is_for(&canonical("kubelint")));
    assert!(!flat(&bin).is_for(&canonical("bin/nested/kubelint")));
    let missing = root.join("gone").display().to_string();
    assert!(!flat(&missing).is_for(&canonical("bin/kubelint")));
    let _ = fs::remove_dir_all(&root);
}
