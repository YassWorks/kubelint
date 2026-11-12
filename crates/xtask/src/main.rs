use std::path::{Path, PathBuf};
use std::process::ExitCode;

const USAGE: &str = "usage: cargo xtask gen-rule-docs | gen-corpus [--objects <N>] [<DIR>]";
const CORPUS_OBJECTS: usize = 10_000;
const CORPUS_DIR: &str = "target/bench-corpus";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let result = match args.split_first() {
        Some((task, [])) if task == "gen-rule-docs" => gen_rule_docs(),
        Some((task, rest)) if task == "gen-corpus" => match corpus_args(rest) {
            Some((objects, dir)) => gen_corpus(objects, &dir),
            None => return usage(),
        },
        _ => return usage(),
    };
    if let Err(error) = result {
        eprintln!("error: {error}");
        return ExitCode::FAILURE;
    }
    ExitCode::SUCCESS
}

fn usage() -> ExitCode {
    eprintln!("{USAGE}");
    ExitCode::from(2)
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

fn corpus_args(args: &[String]) -> Option<(usize, PathBuf)> {
    let mut objects = CORPUS_OBJECTS;
    let mut dir = None;
    let mut args = args.iter();
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--objects" => objects = args.next()?.parse().ok()?,
            _ if dir.is_none() => dir = Some(PathBuf::from(arg)),
            _ => return None,
        }
    }
    Some((
        objects,
        dir.unwrap_or_else(|| xtask::repository_root().join(CORPUS_DIR)),
    ))
}

/// Replaces the `corpus-*.yaml` files of `dir` and leaves its other files alone.
fn gen_corpus(objects: usize, dir: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(dir)?;
    for entry in std::fs::read_dir(dir)? {
        let path = entry?.path();
        let name = path.file_name().unwrap_or_default().to_string_lossy();
        if name.starts_with("corpus-") && name.ends_with(".yaml") {
            std::fs::remove_file(&path)?;
        }
    }
    let files = xtask::corpus(objects);
    for (name, content) in &files {
        std::fs::write(dir.join(name), content)?;
    }
    println!(
        "wrote {objects} objects in {} files to {}",
        files.len(),
        dir.display()
    );
    Ok(())
}
