#[test]
fn rule_pages_are_up_to_date() {
    let root = xtask::repository_root();
    let stale: Vec<String> = xtask::rule_pages()
        .into_iter()
        .filter(|(path, content)| {
            std::fs::read_to_string(root.join(path)).ok().as_deref() != Some(content.as_str())
        })
        .map(|(path, _)| path.display().to_string())
        .collect();
    assert!(
        stale.is_empty(),
        "stale rule pages, run `cargo xtask gen-rule-docs`: {stale:?}"
    );
}

#[test]
fn every_rule_page_belongs_to_a_rule() {
    let orphans = xtask::orphan_rule_pages().expect("docs/rules can be read");
    assert!(
        orphans.is_empty(),
        "pages without a rule, run `cargo xtask gen-rule-docs`: {orphans:?}"
    );
}
