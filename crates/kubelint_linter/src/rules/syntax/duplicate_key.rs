//! KL0002 `duplicate-key`.

use std::collections::hash_map::Entry;
use std::collections::{HashMap, HashSet};
use std::fmt::Write as _;

use kubelint_diagnostics::Severity;
use kubelint_source::TextRange;
use kubelint_yaml::{Node, NodeKind};

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
        let mut found = Vec::new();
        find_duplicates(cx.object().node(), &mut String::new(), &mut found);
        // An alias (`*name`) repeats its anchor's nodes with the anchor's ranges: report
        // each duplicate once.
        let mut reported = HashSet::new();
        for duplicate in found {
            if !reported.insert(duplicate.repeat) {
                continue;
            }
            let message = if duplicate.path.is_empty() {
                format!("duplicate key `{}`", duplicate.key)
            } else {
                format!("duplicate key `{}` in `{}`", duplicate.key, duplicate.path)
            };
            cx.report(
                cx.diagnostic(duplicate.repeat, message)
                    .with_secondary(cx.label(duplicate.first).with_message("first defined here"))
                    .with_help(format!(
                        "keep one `{}`: Kubernetes either rejects the object or uses only the \
                         last value",
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

/// Collects the duplicate keys of `node` and every node below it, in source order.
/// `path` is the location of `node`; it is restored before returning.
fn find_duplicates(node: &Node, path: &mut String, found: &mut Vec<Duplicate>) {
    match &node.kind {
        NodeKind::Mapping(entries) => {
            let mut first_seen: HashMap<&str, TextRange> = HashMap::new();
            for entry in entries {
                let Some(key) = comparable_key(entry) else {
                    continue;
                };
                match first_seen.entry(key) {
                    Entry::Vacant(vacant) => {
                        vacant.insert(entry.key.range);
                    }
                    Entry::Occupied(first) => found.push(Duplicate {
                        key: key.to_owned(),
                        path: path.clone(),
                        first: *first.get(),
                        repeat: entry.key.range,
                    }),
                }
            }
            for entry in entries {
                let len = path.len();
                push_key(path, entry.key_str().unwrap_or("?"));
                find_duplicates(&entry.value, path, found);
                path.truncate(len);
            }
        }
        NodeKind::Sequence(items) => {
            for (i, item) in items.iter().enumerate() {
                let len = path.len();
                let _ = write!(path, "[{i}]");
                find_duplicates(item, path, found);
                path.truncate(len);
            }
        }
        NodeKind::Scalar(_) | NodeKind::Null => {}
    }
}

/// The key text used to compare entries, or `None` for keys this rule does not compare:
/// mappings or sequences used as keys, and YAML merge keys (`<<`), which may repeat.
fn comparable_key(entry: &kubelint_yaml::Entry) -> Option<&str> {
    entry.key_str().filter(|key| *key != "<<")
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
