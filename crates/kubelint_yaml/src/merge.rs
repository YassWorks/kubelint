//! Merge key (`<<`) expansion, with the precedence `go-yaml` v2 gives it. `kubectl` and
//! the API server read manifests with it.

use std::collections::{HashMap, HashSet};

use kubelint_source::{TextRange, TextSize};

use crate::{Entry, Node, NodeKind};

/// Replaces the merge entries of a mapping (`merges` holds their sorted indices first) by the
/// entries they merge, in place, then drops what a later entry overrides. Fails with the
/// offset of the first `<<` whose value is not a mapping or a sequence of mappings.
pub(crate) fn expand(
    entries: Vec<Entry>,
    merges: &[(usize, TextRange)],
) -> Result<Vec<Entry>, TextSize> {
    if merges.is_empty() {
        return Ok(entries);
    }
    // Each entry comes with its source: 0 for written here, else the merge source's number.
    let mut out = Vec::with_capacity(entries.len());
    let mut origins: HashMap<(u32, u32), u32> = HashMap::new();
    let mut source_number = 0;
    for (i, entry) in entries.into_iter().enumerate() {
        if merges.binary_search_by_key(&i, |&(at, _)| at).is_err() {
            out.push((entry, 0));
            continue;
        }
        let at = entry.key.range.start();
        for source in sources(entry.value).ok_or(at)? {
            source_number += 1;
            for mut e in source {
                let next = u32::try_from(origins.len() + 1).unwrap_or(u32::MAX);
                e.origin = *origins.entry((source_number, e.origin)).or_insert(next);
                out.push((e, source_number));
            }
        }
    }
    Ok(drop_overridden(out))
}

/// The mappings a merge value stands for. A sequence lists its items backwards: the last
/// entry of a key wins, and the earlier item of a sequence has to win.
fn sources(value: Node) -> Option<Vec<Vec<Entry>>> {
    match value.kind {
        NodeKind::Mapping(entries) => Some(vec![entries]),
        NodeKind::Sequence(items) => items
            .into_iter()
            .rev()
            .map(|item| match item.kind {
                NodeKind::Mapping(entries) => Some(entries),
                _ => None,
            })
            .collect(),
        _ => None,
    }
}

/// Drops each entry that a later entry of another source overrides, unless its own source
/// repeats the key: those repeats stay for KL0002.
fn drop_overridden(entries: Vec<(Entry, u32)>) -> Vec<Entry> {
    let mut per_source: HashMap<(u32, &str), usize> = HashMap::new();
    for (entry, source) in &entries {
        if let Some(key) = entry.key_str() {
            *per_source.entry((*source, key)).or_default() += 1;
        }
    }
    let mut dropped = vec![false; entries.len()];
    let mut later: HashMap<&str, HashSet<u32>> = HashMap::new();
    for (i, (entry, source)) in entries.iter().enumerate().rev() {
        let Some(key) = entry.key_str() else { continue };
        let sources = later.entry(key).or_default();
        let overridden = sources.iter().any(|s| s != source);
        dropped[i] = overridden && per_source[&(*source, key)] == 1;
        sources.insert(*source);
    }
    entries
        .into_iter()
        .zip(dropped)
        .filter_map(|((entry, _), dropped)| (!dropped).then_some(entry))
        .collect()
}
