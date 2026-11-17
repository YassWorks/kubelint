//! Which files a directory named on the command line yields when ignore files hide them.

use std::path::{Path, PathBuf};

use assert_cmd::Command;

const POD: &str = include_str!("fixtures/pod.yaml");

/// A temporary project with `gen/a.yaml`, `deploy/b.yaml`, `deploy/c.tmp.yaml`,
/// `sub/node_modules/x.yaml`, `sub/y.yaml` and the given root `.gitignore`. Removed on drop.
struct Project(PathBuf);

impl Project {
    fn new(name: &str, gitignore: &str) -> Self {
        let project = Self::empty(name, gitignore);
        for file in [
            "gen/a.yaml",
            "deploy/b.yaml",
            "deploy/c.tmp.yaml",
            "sub/node_modules/x.yaml",
            "sub/y.yaml",
        ] {
            project.write(file, POD);
        }
        project
    }

    /// A project with only the root `.gitignore`.
    fn empty(name: &str, gitignore: &str) -> Self {
        let root = std::env::temp_dir().join(format!("kubelint-gitignore-{name}"));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).expect("mkdir");
        let project = Self(root);
        project.write(".gitignore", gitignore);
        project
    }

    fn write(&self, file: &str, text: &str) {
        let path = self.0.join(file);
        std::fs::create_dir_all(path.parent().expect("has a parent")).expect("mkdir");
        std::fs::write(path, text).expect("write");
    }

    fn run(&self, dir: &str, args: &[&str]) -> String {
        run_in(&self.0.join(dir), args, &[])
    }
}

impl Drop for Project {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn run_in(dir: &Path, args: &[&str], env: &[(&str, &Path)]) -> String {
    let output = Command::cargo_bin("kubelint")
        .expect("binary is built")
        .current_dir(dir)
        .args(args)
        .envs(env.iter().copied())
        .output()
        .expect("binary runs");
    format!(
        "exit: {}\n--- stdout\n{}--- stderr\n{}",
        output.status.code().unwrap_or(-1),
        summary_without_timing(&String::from_utf8_lossy(&output.stdout)),
        String::from_utf8_lossy(&output.stderr)
    )
}

fn summary_without_timing(text: &str) -> String {
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
fn a_directory_hidden_by_a_star_pattern_is_linted_when_named() {
    let project = Project::new("star", "gen/*\n");
    insta::assert_snapshot!(project.run(".", &["check", "gen"]));
}

#[test]
fn a_directory_hidden_by_a_double_star_pattern_is_linted_when_named() {
    let project = Project::new("double-star", "gen/**\n");
    insta::assert_snapshot!(project.run(".", &["check", "gen"]));
}

#[test]
fn a_directory_hidden_by_an_anchored_pattern_is_linted_when_named() {
    let project = Project::new("anchored", "/gen/*\n");
    insta::assert_snapshot!(project.run(".", &["check", "gen"]));
}

#[test]
fn an_ignored_directory_is_linted_when_named_with_a_trailing_slash() {
    let project = Project::new("slash", "gen/\n");
    insta::assert_snapshot!(project.run(".", &["check", "gen/"]));
}

#[test]
fn an_ignored_directory_is_skipped_when_searching_its_parent() {
    let project = Project::new("parent", "gen/*\n");
    insta::assert_snapshot!(project.run(".", &["check", "."]));
}

#[test]
fn a_partly_hidden_directory_keeps_its_hidden_files_skipped() {
    let project = Project::new("partly", "*.tmp.yaml\n");
    insta::assert_snapshot!(project.run(".", &["check", "deploy"]));
}

#[test]
fn an_ignore_file_inside_the_named_directory_still_applies() {
    let project = Project::new("inside", "gen/*\n");
    project.write("gen/.gitignore", "a.yaml\n");
    insta::assert_snapshot!(project.run(".", &["check", "gen"]));
}

#[test]
fn a_subdirectory_search_keeps_the_root_ignore_rules() {
    let project = Project::new("subdir", "node_modules/\n");
    insta::assert_snapshot!(project.run("sub", &["check", "."]));
}

#[test]
fn a_directory_whose_only_yaml_is_in_an_ignored_subdirectory_is_still_skipped() {
    let project = Project::empty("only-ignored-below", "node_modules/\n");
    project.write("app/node_modules/pkg/x.yaml", POD);
    insta::assert_snapshot!(project.run(".", &["check", "app"]));
}

#[test]
fn a_dot_ignore_file_above_does_not_apply_to_a_directory_it_hides_entirely() {
    let project = Project::new("dot-ignore", "");
    project.write(".ignore", "gen/*\n");
    insta::assert_snapshot!(project.run(".", &["check", "gen"]));
}

#[test]
fn the_repository_exclude_file_does_not_apply_to_a_directory_it_hides_entirely() {
    let project = Project::new("info-exclude", "");
    project.write(".git/info/exclude", "gen/*\n");
    insta::assert_snapshot!(project.run(".", &["check", "gen"]));
}

#[test]
fn the_global_excludes_still_apply_to_a_named_directory() {
    let project = Project::new("global", "");
    project.write("config/git/ignore", "gen/*\n");
    let config = project.0.join("config");
    insta::assert_snapshot!(run_in(
        &project.0,
        &["check", "gen"],
        &[("XDG_CONFIG_HOME", &config), ("HOME", &config)]
    ));
}

#[test]
fn several_named_directories_are_each_searched_on_their_own() {
    let project = Project::new("several", "gen/*\n");
    insta::assert_snapshot!(project.run(".", &["check", "gen", "deploy"]));
}

#[test]
fn configured_exclude_patterns_still_skip_a_directory_found_again() {
    let project = Project::new("config-exclude", "gen/*\n");
    project.write("kubelint.toml", "[files]\nexclude = [\"gen/**\"]\n");
    insta::assert_snapshot!(project.run(".", &["check", "gen"]));
}

#[test]
fn helm_chart_templates_are_still_skipped_in_a_directory_found_again() {
    let project = Project::empty("helm", "templates/*\n");
    project.write(
        "chart/Chart.yaml",
        "apiVersion: v2\nname: chart\nversion: 0.1.0\n",
    );
    project.write("chart/templates/deploy.yaml", POD);
    insta::assert_snapshot!(project.run(".", &["check", "chart/templates"]));
}

#[test]
fn a_hidden_file_next_to_a_visible_one_stays_skipped() {
    let project = Project::new("mixed", "gen/*.yaml\n!gen/keep.yaml\n");
    project.write("gen/keep.yaml", POD);
    insta::assert_snapshot!(project.run(".", &["check", "gen"]));
}
