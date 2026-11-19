use std::error::Error;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};

use kubelint_schema::KubernetesVersion;
use xtask::e2e::{self, Outcome};
use xtask::schemas;

const USAGE: &str = "usage: cargo xtask gen-rule-docs | gen-corpus [--objects <N>] [<DIR>] \
                     | import-schemas <KUBERNETES_CHECKOUT> | e2e [<CODE>...]";
const CORPUS_OBJECTS: usize = 10_000;
const CORPUS_DIR: &str = "target/bench-corpus";
const OPENAPI_DIR: &str = "api/openapi-spec/v3";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let result: Result<(), Box<dyn Error>> = match args.split_first() {
        Some((task, [])) if task == "gen-rule-docs" => gen_rule_docs().map_err(Into::into),
        Some((task, rest)) if task == "gen-corpus" => match corpus_args(rest) {
            Some((objects, dir)) => gen_corpus(objects, &dir).map_err(Into::into),
            None => return usage(),
        },
        Some((task, [checkout])) if task == "import-schemas" => import_schemas(Path::new(checkout)),
        Some((task, codes)) if task == "e2e" => run_e2e(codes),
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

/// Reads `api/openapi-spec/v3` of a Kubernetes checkout at a release tag and writes the
/// schema of that tag's minor version, replacing an earlier import of the same one.
fn import_schemas(checkout: &Path) -> Result<(), Box<dyn Error>> {
    let tag = git(checkout, &["describe", "--tags", "--exact-match", "HEAD"])?;
    let version = schemas::minor_version(&tag)
        .ok_or_else(|| format!("`{tag}` is not a release tag such as `v1.37.2`"))?
        .to_owned();
    let changes = git(checkout, &["status", "--porcelain", "--", OPENAPI_DIR])?;
    if !changes.is_empty() {
        return Err(format!("{OPENAPI_DIR} differs from {tag}:\n{changes}").into());
    }
    let commit = git(checkout, &["rev-parse", "HEAD"])?;
    let input = checkout.join(OPENAPI_DIR);
    let read = |path: &Path| {
        std::fs::read_to_string(path).map_err(|error| format!("{}: {error}", path.display()))
    };
    let mut documents = Vec::new();
    for path in schemas::json_files(&input)
        .map_err(|error| format!("cannot read {}: {error}", input.display()))?
    {
        let name = path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned();
        documents.push((name, read(&path)?));
    }
    let text = schemas::import(&documents)?;
    let output = xtask::repository_root().join(schemas::SCHEMAS_DIR);
    let sources_path = output.join("sources.toml");
    let mut sources = schemas::read_sources(&read(&sources_path)?)
        .map_err(|error| format!("{}: {error}", sources_path.display()))?;
    println!(
        "imported {tag} from {} files into {version}.json ({} bytes)",
        documents.len(),
        text.len()
    );
    std::fs::write(output.join(format!("{version}.json")), &text)?;
    sources.insert(version, schemas::Source { tag, commit });
    std::fs::write(&sources_path, schemas::write_sources(&sources))?;
    Ok(())
}

fn git(dir: &Path, args: &[&str]) -> Result<String, Box<dyn Error>> {
    let output = Command::new("git").arg("-C").arg(dir).args(args).output()?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("`git {}` failed: {}", args.join(" "), stderr.trim()).into());
    }
    Ok(String::from_utf8(output.stdout)?.trim().to_owned())
}

/// Applies every case of the Pod Security rules (or of `codes`) to the API server that
/// `KUBECONFIG` names and fails when one disagrees with kubelint.
fn run_e2e(codes: &[String]) -> Result<(), Box<dyn Error>> {
    let version = server_version()?;
    let root = xtask::repository_root();
    let dir = root.join("target/e2e/cases");
    std::fs::create_dir_all(&dir)?;
    let selected: Vec<_> = e2e::POD_SECURITY
        .iter()
        .filter(|rule| codes.is_empty() || codes.iter().any(|code| code == rule.code))
        .collect();
    if let Some(code) = codes
        .iter()
        .find(|code| !selected.iter().any(|rule| rule.code == *code))
    {
        return Err(format!("{code} has no end to end check").into());
    }
    let mut mismatches = 0;
    for check in selected {
        let rule = e2e::rule(check.code).ok_or_else(|| format!("no rule {}", check.code))?;
        let path = e2e::fixture_path(rule);
        let cases = e2e::cases(rule, &std::fs::read_to_string(root.join(&path))?, version);
        let mut namespaces = vec!["default"];
        namespaces.extend(cases.iter().filter_map(|case| case.namespace.as_deref()));
        for namespace in namespaces {
            let manifest = format!(
                r#"{{"apiVersion":"v1","kind":"Namespace","metadata":{{"name":"{namespace}","labels":{{"pod-security.kubernetes.io/enforce":"{}"}}}}}}"#,
                check.level
            );
            let outcome = kubectl(&["apply", "-f", "-"], Some(&manifest))?;
            if !outcome.accepted {
                return Err(
                    format!("cannot create namespace {namespace}: {}", outcome.output).into(),
                );
            }
        }
        for case in &cases {
            let place = format!("{path}:{}", case.line);
            if let Some(reason) = &case.skip {
                println!("skip     {place}: {reason}");
                continue;
            }
            let file = dir.join(format!("{}-{}.yaml", check.code, case.line));
            std::fs::write(&file, &case.text)?;
            let file = file.to_string_lossy();
            let outcome = kubectl(&["apply", "--dry-run=server", "-f", &file], None)?;
            match e2e::pod_security_mismatch(check, case, &outcome) {
                None => println!("ok       {place}"),
                Some(reason) => {
                    mismatches += 1;
                    println!("mismatch {place}: {reason}\n{}", outcome.output.trim_end());
                }
            }
        }
    }
    if mismatches > 0 {
        return Err(format!(
            "{mismatches} case(s) disagree with the Kubernetes {version} API server"
        )
        .into());
    }
    Ok(())
}

fn server_version() -> Result<KubernetesVersion, Box<dyn Error>> {
    let outcome = kubectl(&["get", "--raw", "/version"], None)?;
    let reply: serde_json::Value = serde_json::from_str(&outcome.output)
        .map_err(|_| format!("no API server answered: {}", outcome.output.trim()))?;
    let git_version = reply["gitVersion"].as_str().unwrap_or_default();
    KubernetesVersion::parse(git_version).ok_or_else(|| {
        format!("kubelint has no schema for the server's version {git_version}").into()
    })
}

fn kubectl(args: &[&str], input: Option<&str>) -> Result<Outcome, Box<dyn Error>> {
    let mut child = Command::new("kubectl")
        .args(args)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|error| format!("cannot run kubectl: {error}"))?;
    if let (Some(input), Some(mut stdin)) = (input, child.stdin.take()) {
        std::io::Write::write_all(&mut stdin, input.as_bytes())?;
    }
    let output = child.wait_with_output()?;
    Ok(Outcome {
        accepted: output.status.success(),
        output: format!(
            "{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        ),
    })
}
