//! `kubelint self update` refusing to run, which it decides before any download. A receipt
//! that names another folder shows which receipt was read.

use std::path::{Path, PathBuf};

use assert_cmd::Command;

/// A temporary folder holding the home folder (`home`), `LOCALAPPDATA` (`local`) and a
/// folder to use as `XDG_CONFIG_HOME` (`xdg`).
struct Root(PathBuf);

impl Root {
    fn new(name: &str) -> Self {
        let root = std::env::temp_dir().join(format!("kubelint-self-update-{name}"));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).expect("mkdir");
        Self(root)
    }

    /// Where the installer writes its receipt when `XDG_CONFIG_HOME` is not set.
    fn default_config(&self) -> PathBuf {
        if cfg!(windows) {
            self.0.join("local")
        } else {
            self.0.join("home").join(".config")
        }
    }

    fn xdg(&self) -> PathBuf {
        self.0.join("xdg")
    }

    /// A receipt for an install into `folder` under this root.
    fn receipt_for(&self, folder: &str) -> String {
        serde_json::json!({
            "install_prefix": self.0.join(folder),
            "install_layout": "flat",
        })
        .to_string()
    }

    /// Runs `kubelint self update` with `XDG_CONFIG_HOME` set to `xdg` (unset for `None`)
    /// and renders exit code, stdout and stderr, with the root folder and the binary's
    /// path replaced so snapshots are stable.
    fn run(&self, xdg: Option<&Path>) -> String {
        let exe = assert_cmd::cargo::cargo_bin("kubelint");
        let mut command = Command::new(&exe);
        command
            .args(["self", "update"])
            .env("HOME", self.0.join("home"))
            .env("LOCALAPPDATA", self.0.join("local"));
        match xdg {
            Some(dir) => command.env("XDG_CONFIG_HOME", dir),
            None => command.env_remove("XDG_CONFIG_HOME"),
        };
        let output = command.output().expect("binary runs");
        let text = format!(
            "exit: {}\n--- stdout\n{}--- stderr\n{}",
            output.status.code().unwrap_or(-1),
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        let text = text
            .replace(&exe.display().to_string(), "[EXE]")
            .replace(&self.0.display().to_string(), "[ROOT]");
        text.replace('\\', "/")
    }
}

impl Drop for Root {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn write_receipt(config: &Path, text: &str) {
    let dir = config.join("kubelint");
    std::fs::create_dir_all(&dir).expect("mkdir");
    std::fs::write(dir.join("kubelint-receipt.json"), text).expect("write");
}

#[test]
fn without_a_receipt_it_refuses() {
    let root = Root::new("no-receipt");
    insta::assert_snapshot!(root.run(None));
}

#[test]
fn with_a_receipt_it_cannot_read_it_refuses() {
    let root = Root::new("bad-receipt");
    write_receipt(&root.xdg(), r#"{"install_prefix":"#);
    insta::assert_snapshot!(root.run(Some(&root.xdg())));
}

#[test]
fn with_a_receipt_for_another_folder_it_refuses() {
    let root = Root::new("other-folder");
    write_receipt(&root.default_config(), &root.receipt_for("bin"));
    insta::assert_snapshot!(root.run(None));
}

#[test]
fn the_receipt_in_xdg_config_home_comes_first() {
    let root = Root::new("xdg-first");
    write_receipt(&root.default_config(), &root.receipt_for("default-bin"));
    write_receipt(&root.xdg(), &root.receipt_for("xdg-bin"));
    insta::assert_snapshot!(root.run(Some(&root.xdg())));
}

#[test]
fn an_empty_xdg_config_home_or_one_without_a_receipt_is_skipped() {
    let root = Root::new("xdg-skipped");
    write_receipt(&root.default_config(), &root.receipt_for("default-bin"));
    let empty = root.run(Some(Path::new("")));
    assert_eq!(root.run(Some(&root.xdg())), empty);
    insta::assert_snapshot!(empty);
}
