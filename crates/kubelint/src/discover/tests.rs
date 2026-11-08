use super::*;

/// A temporary directory with the given files (empty), removed on drop.
struct Tree(PathBuf);

impl Tree {
    fn new(name: &str, files: &[&str]) -> Self {
        let root = std::env::temp_dir().join(format!("kubelint-discover-{name}"));
        let _ = std::fs::remove_dir_all(&root);
        for file in files {
            let path = root.join(file);
            std::fs::create_dir_all(path.parent().expect("has a parent")).expect("mkdir");
            std::fs::write(path, "").expect("write");
        }
        Self(root)
    }
}

impl Drop for Tree {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn detector() -> HelmDetector {
    HelmDetector {
        cwd: None,
        is_chart: HashMap::new(),
    }
}

#[test]
fn templates_and_chart_yaml_of_a_chart_are_skipped() {
    let tree = Tree::new(
        "chart",
        &[
            "web/Chart.yaml",
            "web/values.yaml",
            "web/templates/deploy.yaml",
            "web/templates/sub/svc.yaml",
            "web/crds/crd.yaml",
        ],
    );
    let web = tree.0.join("web");
    let mut helm = detector();
    assert_eq!(helm.chart_of(&web.join("Chart.yaml")), Some(web.clone()));
    let deploy = web.join("templates/deploy.yaml");
    assert_eq!(helm.chart_of(&deploy), Some(web.clone()));
    let nested = web.join("templates/sub/svc.yaml");
    assert_eq!(helm.chart_of(&nested), Some(web.clone()));
    assert_eq!(helm.chart_of(&web.join("values.yaml")), None);
    assert_eq!(helm.chart_of(&web.join("crds/crd.yaml")), None);
}

#[test]
fn templates_outside_a_chart_are_linted() {
    let tree = Tree::new("no-chart", &["app/templates/deploy.yaml"]);
    let mut helm = detector();
    assert_eq!(
        helm.chart_of(&tree.0.join("app/templates/deploy.yaml")),
        None
    );
}

#[test]
fn chart_is_shown_relative_to_the_working_directory() {
    let tree = Tree::new("relative", &["web/Chart.yaml", "web/templates/a.yaml"]);
    let mut helm = HelmDetector {
        cwd: Some(tree.0.clone()),
        is_chart: HashMap::new(),
    };
    let chart = helm.chart_of(Path::new("./web/templates/a.yaml"));
    assert_eq!(chart, Some(PathBuf::from("web")));
    assert_eq!(
        helm.chart_of(Path::new("web/Chart.yaml")),
        Some("web".into())
    );

    // Run from inside the chart, and from inside `templates/`.
    helm.cwd = Some(tree.0.join("web"));
    let chart = helm.chart_of(Path::new("templates/a.yaml"));
    assert_eq!(chart, Some(PathBuf::from(".")));
    helm.cwd = Some(tree.0.join("web/templates"));
    let chart = helm.chart_of(Path::new("a.yaml"));
    assert_eq!(chart, Some(PathBuf::from("..")));
}
