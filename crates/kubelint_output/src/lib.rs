//! Renders diagnostics for people (rustc-style snippets via `annotate-snippets`) and for
//! machines (JSON, SARIF, GitHub Actions workflow commands). The formats are documented in
//! `docs/user/output-formats.md`.
//!
//! Allowed internal dependencies: `kubelint_source`, `kubelint_diagnostics`.
//! See `docs/architecture.md`.

use kubelint_diagnostics::Diagnostic;
use kubelint_source::{FileId, SourceFile};

mod github;
mod human;
mod json;
mod sarif;

pub use github::GithubRenderer;
pub use human::{HumanRenderer, Summary};
pub use json::{JSON_SCHEMA_VERSION, JsonRenderer};
pub use sarif::{SarifRenderer, SarifRule};

/// `None` when a label points at a file that is not part of the run (a bug elsewhere).
fn file(files: &[SourceFile], id: FileId) -> Option<&SourceFile> {
    files.get(usize::try_from(id.0).ok()?)
}

/// The file's path with `/` separators, so output looks the same on every platform.
fn path_string(file: &SourceFile) -> String {
    file.path().to_string_lossy().replace('\\', "/")
}

/// The message, with the help as a second line (`help: ...`) when there is one.
fn message_with_help(diagnostic: &Diagnostic) -> String {
    match &diagnostic.help {
        Some(help) => format!("{}\nhelp: {help}", diagnostic.message),
        None => diagnostic.message.clone(),
    }
}
