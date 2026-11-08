//! `kubelint rules` and `kubelint rule <CODE|NAME>`: what the built-in rules are.

use std::fmt::Write as _;

use kubelint_linter::{RuleMeta, rules};

#[derive(Debug, Clone, Copy, clap::ValueEnum)]
pub enum RulesFormat {
    /// An aligned table.
    Text,
    /// A JSON array, one object per rule.
    Json,
}

fn metas() -> impl Iterator<Item = &'static RuleMeta> {
    rules::ALL.iter().map(|rule| rule.meta())
}

fn on_off(meta: &RuleMeta) -> &'static str {
    if meta.default_enabled { "on" } else { "off" }
}

/// Every rule, in code order.
pub fn list(format: RulesFormat) -> String {
    match format {
        RulesFormat::Text => table(),
        RulesFormat::Json => json(),
    }
}

fn table() -> String {
    let header = ["CODE", "NAME", "CATEGORY", "DEFAULT", "SEVERITY", "SUMMARY"];
    let rows: Vec<[&str; 6]> = metas()
        .map(|meta| {
            [
                meta.code,
                meta.name,
                meta.category.as_str(),
                on_off(meta),
                meta.default_severity.as_str(),
                meta.summary,
            ]
        })
        .collect();
    let mut widths = header.map(str::len);
    for row in &rows {
        for (width, cell) in widths.iter_mut().zip(row) {
            *width = (*width).max(cell.len());
        }
    }
    let mut out = String::new();
    for row in std::iter::once(&header).chain(&rows) {
        let mut line = String::new();
        for (cell, width) in row.iter().zip(widths) {
            let _ = write!(line, "{cell:<width$}  ");
        }
        out.push_str(line.trim_end());
        out.push('\n');
    }
    out
}

fn json() -> String {
    let rules: Vec<serde_json::Value> = metas()
        .map(|meta| {
            serde_json::json!({
                "code": meta.code,
                "name": meta.name,
                "category": meta.category.as_str(),
                "default_enabled": meta.default_enabled,
                "default_severity": meta.default_severity.as_str(),
                "summary": meta.summary,
            })
        })
        .collect();
    let mut out = serde_json::to_string_pretty(&rules).unwrap_or_else(|_| "[]".to_owned());
    out.push('\n');
    out
}

/// The documentation of the rule with this code or name.
pub fn explain(selector: &str) -> Option<String> {
    let meta = metas().find(|meta| meta.code == selector || meta.name == selector)?;
    let enabled = if meta.default_enabled {
        "Enabled by default"
    } else {
        "Disabled by default, select it by code or name to run it"
    };
    Some(format!(
        "# {} `{}`\n\n{}\n\nCategory: `{}`. Severity: `{}`. {enabled}.\n\n{}\n",
        meta.code,
        meta.name,
        meta.summary,
        meta.category.as_str(),
        meta.default_severity.as_str(),
        meta.explanation.trim_end_matches('\n'),
    ))
}

#[cfg(test)]
mod tests;
