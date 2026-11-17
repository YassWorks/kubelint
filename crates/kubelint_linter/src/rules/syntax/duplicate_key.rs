//! KL0002 `duplicate-key`.

use std::collections::hash_map::Entry;
use std::collections::{HashMap, HashSet};
use std::fmt::Write as _;

use kubelint_diagnostics::Severity;
use kubelint_source::TextRange;
use kubelint_yaml::{Node, NodeKind};

use crate::rules::object_subject;
use crate::{Category, ObjectContext, Rule, RuleMeta};

/// Reports a key that appears more than once in the same mapping.
pub struct DuplicateKey;

static META: RuleMeta = RuleMeta {
    code: "KL0002",
    name: "duplicate-key",
    category: Category::Syntax,
    default_severity: Severity::Error,
    default_enabled: true,
    summary: "The same key appears twice in one mapping.",
    explanation: include_str!("duplicate_key.md"),
};

impl Rule for DuplicateKey {
    fn meta(&self) -> &'static RuleMeta {
        &META
    }

    fn check_object(&self, cx: &mut ObjectContext<'_>) {
        let found = find_duplicates(cx.object().node());
        // An alias (`*name`) repeats its anchor's nodes with the anchor's ranges: report
        // each duplicate once.
        let mut reported = HashSet::new();
        let subject = object_subject(cx.object());
        for duplicate in found {
            if !reported.insert(duplicate.repeat) {
                continue;
            }
            let message = if duplicate.path.is_empty() {
                format!(
                    "duplicate key `{}` at the top level of {subject}",
                    duplicate.key
                )
            } else {
                format!("duplicate key `{}` in `{}`", duplicate.key, duplicate.path)
            };
            cx.report(
                cx.diagnostic(duplicate.repeat, message)
                    .with_secondary(cx.label(duplicate.first).with_message("first defined here"))
                    .with_help(format!(
                        "keep one `{}`: `kubectl` uses only the last value and does not warn",
                        duplicate.key
                    )),
            );
        }
    }
}

/// A key seen again in the same mapping.
struct Duplicate {
    key: String,
    /// Where the mapping is, like `spec.containers[0]`; empty for the object itself.
    path: String,
    first: TextRange,
    repeat: TextRange,
}

/// A pending step of the walk in `find_duplicates`.
enum Step<'a> {
    Visit(&'a Node),
    Key(&'a str),
    Index(usize),
    /// Cuts the path back to this length once a child is done.
    Restore(usize),
}

/// Collects the duplicate keys of `root` and every node below it, in source order.
/// The walk keeps its own stack so that nesting depth does not use the call stack.
fn find_duplicates(root: &Node) -> Vec<Duplicate> {
    let mut found = Vec::new();
    let mut path = String::new();
    let mut stack = vec![Step::Visit(root)];
    while let Some(step) = stack.pop() {
        match step {
            Step::Key(key) => push_key(&mut path, key),
            Step::Index(i) => {
                let _ = write!(path, "[{i}]");
            }
            Step::Restore(len) => path.truncate(len),
            Step::Visit(node) => {
                let len = path.len();
                match &node.kind {
                    NodeKind::Mapping(entries) => {
                        collect_repeats(entries, &path, &mut found);
                        for entry in entries.iter().rev() {
                            stack.push(Step::Restore(len));
                            stack.push(Step::Visit(&entry.value));
                            stack.push(Step::Key(entry.key_str().unwrap_or("?")));
                        }
                    }
                    NodeKind::Sequence(items) => {
                        for (i, item) in items.iter().enumerate().rev() {
                            stack.push(Step::Restore(len));
                            stack.push(Step::Visit(item));
                            stack.push(Step::Index(i));
                        }
                    }
                    NodeKind::Scalar(_) | NodeKind::Null => {}
                }
            }
        }
    }
    found
}

/// Collects the keys of one mapping that repeat an earlier key, in source order.
fn collect_repeats(entries: &[kubelint_yaml::Entry], path: &str, found: &mut Vec<Duplicate>) {
    // Entries a merge brought in repeat a key only within their own merge source.
    let mut first_seen: HashMap<(&str, u32), TextRange> = HashMap::new();
    for entry in entries {
        let Some(key) = entry.key_str() else {
            continue;
        };
        match first_seen.entry((key, entry.origin)) {
            Entry::Vacant(vacant) => {
                vacant.insert(entry.key.range);
            }
            Entry::Occupied(first) => found.push(Duplicate {
                key: key.to_owned(),
                path: path.to_owned(),
                first: *first.get(),
                repeat: entry.key.range,
            }),
        }
    }
}

/// Appends `key` to a path: `.key` for simple keys, `["key"]` for keys with dots or other
/// characters that would make the path ambiguous.
fn push_key(path: &mut String, key: &str) {
    let simple = !key.is_empty()
        && key
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_');
    if simple {
        if !path.is_empty() {
            path.push('.');
        }
        path.push_str(key);
    } else {
        let _ = write!(path, "[{key:?}]");
    }
}

#[cfg(test)]
mod tests;
