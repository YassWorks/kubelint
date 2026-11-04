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
    run_in(project(), args, stdin)
}

/// Like [`run`], with `dir` as the working directory.
fn run_in(dir: &Path, args: &[&str], stdin: Option<&str>) -> String {
    let mut cmd = Command::cargo_bin("kubelint").expect("binary is built");
    cmd.current_dir(dir).args(args).env_remove("NO_COLOR");
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

#[test]
fn helm_chart_templates_and_chart_yaml_are_skipped_even_when_named() {
    insta::assert_snapshot!(run(
        &["check", "charts/web/Chart.yaml", "charts/web/templates/service.yaml"],
        None
    ), @r"
    exit: 0
    --- stdout
    All checks passed. Checked 0 files and 0 objects, skipped 2 files in [TIME].
    --- stderr
    note: skipped 2 files of the Helm chart `charts/web`; kubelint checks rendered manifests, not templates
      = help: run `helm template charts/web | kubelint check -`
    ");
}

#[test]
fn helm_chart_is_found_from_inside_its_templates() {
    let templates = project().join("charts/web/templates");
    insta::assert_snapshot!(run_in(&templates, &["check"], None), @r"
    exit: 0
    --- stdout
    All checks passed. Checked 0 files and 0 objects, skipped 2 files in [TIME].
    --- stderr
    note: skipped 2 files of the Helm chart `..`; kubelint checks rendered manifests, not templates
      = help: run `helm template .. | kubelint check -`
    ");
}

/// A project with a `kubelint.toml` that ignores KL1001, downgrades KL1002 to a warning,
/// fails only on errors and excludes `generated/`.
fn configured() -> &'static Path {
    Path::new(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/configured"
    ))
}

#[test]
fn the_config_file_is_found_and_applied() {
    insta::assert_snapshot!(run_in(configured(), &["check", "--color", "never"], None));
}

#[test]
fn isolated_ignores_the_config_file() {
    let out = run_in(
        configured(),
        &["check", "--isolated", "--color", "never"],
        None,
    );
    assert!(out.starts_with("exit: 1\n"), "{out}");
    assert!(out.contains("generated/broken.yaml"), "{out}");
    assert!(
        out.contains("error[KL1001]") && out.contains("error[KL1002]"),
        "{out}"
    );
}

#[test]
fn flags_replace_the_config_file_values() {
    // `--ignore` replaces the file's list, so KL1001 runs again; `--fail-on warning` makes
    // KL1002, still a warning through the file's `[lint.severity]`, fail the run.
    let out = run_in(
        configured(),
        &[
            "check",
            "--select=KL1001,missing-kind",
            "--ignore=KL0002",
            "--fail-on",
            "warning",
        ],
        None,
    );
    assert!(out.starts_with("exit: 1\n"), "{out}");
    assert!(
        out.contains("error[KL1001]") && out.contains("warning[KL1002]"),
        "{out}"
    );
}

#[test]
fn unknown_config_key_is_shown_in_the_file() {
    insta::assert_snapshot!(run_in(
        configured(),
        &[
            "check",
            "--config",
            "bad/unknown-key.toml",
            "--color",
            "never"
        ],
        None
    ));
}

#[test]
fn unknown_selector_in_the_config_is_shown_in_the_file() {
    insta::assert_snapshot!(run_in(
        configured(),
        &[
            "check",
            "--config",
            "bad/unknown-selector.toml",
            "--color",
            "never"
        ],
        None
    ));
}

#[test]
fn unknown_selector_in_a_flag_exits_two() {
    insta::assert_snapshot!(run_in(configured(), &["check", "--select", "KL9"], None));
}

#[test]
fn missing_config_file_exits_two() {
    let out = run_in(configured(), &["check", "--config", "nope.toml"], None);
    assert!(out.starts_with("exit: 2\n"), "{out}");
    assert!(
        out.contains("cannot read the configuration nope.toml"),
        "{out}"
    );
}

#[test]
fn config_and_isolated_conflict() {
    let out = run_in(
        configured(),
        &["check", "--config", "kubelint.toml", "--isolated"],
        None,
    );
    assert!(out.starts_with("exit: 2\n"), "{out}");
}

#[test]
fn excluded_files_are_skipped_even_when_named() {
    insta::assert_snapshot!(run_in(
        configured(),
        &["check", "generated/broken.yaml", "generated"],
        None
    ), @r"
    exit: 0
    --- stdout
    All checks passed. Checked 0 files and 0 objects in [TIME].
    --- stderr
    ");
}

#[test]
fn exclude_patterns_are_relative_to_the_config_file() {
    // Run from `generated/`, the file is still excluded by `generated/**`.
    let out = run_in(&configured().join("generated"), &["check"], None);
    assert!(out.contains("Checked 0 files"), "{out}");
}

#[test]
fn invalid_exclude_pattern_is_shown_in_the_file() {
    insta::assert_snapshot!(run_in(
        configured(),
        &[
            "check",
            "--config",
            "bad/invalid-glob.toml",
            "--color",
            "never"
        ],
        None
    ));
}
