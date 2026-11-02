//! End-to-end tests of the `kubelint` binary.

use std::path::Path;

use assert_cmd::Command;

/// The fixture project, used as the working directory.
fn project() -> &'static Path {
    Path::new(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/project"
    ))
}

/// Runs `kubelint` in the fixture project and renders exit code, stdout and stderr, with
/// the timing replaced so snapshots are stable.
fn run(args: &[&str], stdin: Option<&str>) -> String {
    let mut cmd = Command::cargo_bin("kubelint").expect("binary is built");
    cmd.current_dir(project()).args(args).env_remove("NO_COLOR");
    if let Some(stdin) = stdin {
        cmd.write_stdin(stdin);
    }
    let output = cmd.output().expect("binary runs");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    format!(
        "exit: {}\n--- stdout\n{}--- stderr\n{}",
        output.status.code().unwrap_or(-1),
        without_timing(&stdout),
        stderr
    )
}

/// Replaces ` in 12ms.` or ` in 1.5s.` at the end of the summary line with ` in [TIME].`.
fn without_timing(text: &str) -> String {
    text.lines()
        .map(|line| match line.rfind(" in ") {
            Some(i) if line.starts_with("Found ") || line.starts_with("All checks passed.") => {
                format!("{} in [TIME].\n", &line[..i])
            }
            _ => format!("{line}\n"),
        })
        .collect()
}

#[test]
fn checks_the_current_directory_by_default() {
    insta::assert_snapshot!(run(&["check", "--color", "never"], None));
}

#[test]
fn clean_file_exits_zero() {
    insta::assert_snapshot!(run(&["check", "deploy/web.yaml"], None), @r"
    exit: 0
    --- stdout
    All checks passed. Checked 1 file and 1 object in [TIME].
    --- stderr
    ");
}

#[test]
fn explicit_files_are_linted_whatever_their_extension() {
    // The text file is plain YAML (a string), so it holds no objects and no errors.
    // `deploy/web.yaml` is named twice but checked once.
    insta::assert_snapshot!(run(
        &["check", "notes.txt", "deploy", "deploy/web.yaml"],
        None
    ));
}

#[test]
fn reads_standard_input() {
    insta::assert_snapshot!(run(&["check", "-", "--color=never"], Some("kind: Pod\n")));
}

#[test]
fn missing_path_exits_two() {
    insta::assert_snapshot!(run(&["check", "nope.yaml"], None), @r"
    exit: 2
    --- stdout
    --- stderr
    error: nope.yaml: no such file or directory
    ");
}

#[test]
fn stdin_twice_exits_two() {
    let out = run(&["check", "-", "-"], Some(""));
    assert!(out.starts_with("exit: 2\n"), "{out}");
}

#[test]
fn bad_arguments_exit_two() {
    let out = run(&["check", "--color", "sometimes"], None);
    assert!(out.starts_with("exit: 2\n"), "{out}");
    assert!(out.contains("invalid value 'sometimes'"), "{out}");
}

#[test]
fn version_exits_zero() {
    let out = run(&["--version"], None);
    assert!(out.starts_with("exit: 0\n--- stdout\nkubelint "), "{out}");
}

#[test]
fn color_always_and_never() {
    assert!(run(&["check", "deploy", "--color", "always"], None).contains("\u{1b}["));
    assert!(!run(&["check", "deploy", "--color", "never"], None).contains('\u{1b}'));
    // Output to a pipe is not a terminal, so `auto` means no color.
    assert!(!run(&["check", "deploy"], None).contains('\u{1b}'));
}
