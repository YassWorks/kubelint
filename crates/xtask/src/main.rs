use std::process::ExitCode;

fn main() -> ExitCode {
    if std::env::args().nth(1).as_deref() != Some("gen-rule-docs") {
        eprintln!("usage: cargo xtask gen-rule-docs");
        return ExitCode::from(2);
    }
    if let Err(error) = gen_rule_docs() {
        eprintln!("error: {error}");
        return ExitCode::FAILURE;
    }
    ExitCode::SUCCESS
}

fn gen_rule_docs() -> std::io::Result<()> {
    let root = xtask::repository_root();
    for path in xtask::orphan_rule_pages()? {
        std::fs::remove_file(root.join(path))?;
    }
    for (path, content) in xtask::rule_pages() {
        std::fs::write(root.join(path), content)?;
    }
    Ok(())
}
