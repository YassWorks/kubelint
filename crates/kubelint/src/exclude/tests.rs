use kubelint_source::TextRange;

use super::*;

fn patterns(globs: &[&str]) -> Vec<Spanned<String>> {
    globs
        .iter()
        .map(|g| Spanned {
            value: (*g).to_owned(),
            range: TextRange::default(),
        })
        .collect()
}

/// An absolute path that works on every platform.
fn root() -> PathBuf {
    std::env::temp_dir().join("kubelint-exclude")
}

fn exclude(base: &str, cwd: &str, globs: &[&str]) -> Exclude {
    let root = root();
    Exclude::new(&root.join(base), &root.join(cwd), &patterns(globs)).expect("valid globs")
}

#[test]
fn patterns_are_relative_to_the_config_directory() {
    let exclude = exclude(
        "project",
        "project/deploy",
        &["deploy/generated/**", "*.tmp.yaml"],
    );
    assert!(exclude.matches(Path::new("generated/a.yaml")));
    assert!(exclude.matches(Path::new("./generated/sub/b.yaml")));
    assert!(!exclude.matches(Path::new("web.yaml")));
    // `*` does not cross `/`: only files directly in the base directory.
    assert!(exclude.matches(Path::new("../x.tmp.yaml")));
    assert!(!exclude.matches(Path::new("x.tmp.yaml")));
}

#[test]
fn a_directory_pattern_excludes_everything_below_it() {
    let exclude = exclude("p", "p", &["vendor", "**/testdata"]);
    assert!(exclude.matches(Path::new("vendor/a/b.yaml")));
    assert!(exclude.matches(Path::new("src/x/testdata/c.yaml")));
    assert!(!exclude.matches(Path::new("vendored/a.yaml")));
}

#[test]
fn paths_outside_the_base_are_never_excluded() {
    let exclude = exclude("p/sub", "p", &["**"]);
    assert!(exclude.matches(Path::new("sub/a.yaml")));
    assert!(!exclude.matches(Path::new("a.yaml")));
    assert!(!exclude.matches(&root().join("elsewhere/a.yaml")));
}

#[test]
fn invalid_patterns_are_reported_with_their_index() {
    let root = root();
    let err = Exclude::new(&root, &root, &patterns(&["ok/**", "[a"])).expect_err("invalid");
    assert_eq!(err.index, 1);
    assert!(
        err.message.starts_with("invalid glob pattern `[a`: "),
        "{}",
        err.message
    );
}
