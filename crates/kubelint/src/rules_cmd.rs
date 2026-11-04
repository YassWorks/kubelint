//! `kubelint rules` and `kubelint rule <CODE|NAME>`: what the built-in rules are.

use std::fmt::Write as _;

use kubelint_linter::{RuleMeta, rules};

/// The output format of `kubelint rules`.
#[derive(Debug, Clone, Copy, clap::ValueEnum)]
pub enum RulesFormat {
    /// An aligned table.
    Text,
    /// A JSON array, one object per rule (ADR 0012).
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
        for (i, cell) in row.iter().enumerate() {
            if i + 1 == row.len() {
                line.push_str(cell);
            } else {
                let _ = write!(line, "{cell:<width$}  ", width = widths[i]);
            }
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

/// The documentation of the rule with this code or name, or `None` if there is none.
pub fn explain(selector: &str) -> Option<String> {
    let meta = metas().find(|meta| meta.code == selector || meta.name == selector)?;
    let enabled = if meta.default_enabled {
        "enabled by default"
    } else {
        "disabled by default, select it by code or name to run it"
    };
    Some(
        format!(
            "# {} `{}`\n\n{}\n\nCategory: `{}`. Severity: `{}`. {}.\n\n{}",
            meta.code,
            meta.name,
            meta.summary,
            meta.category.as_str(),
            meta.default_severity.as_str(),
            capitalize(enabled),
            meta.explanation.trim_end_matches('\n'),
        ) + "\n",
    )
}

fn capitalize(text: &str) -> String {
    let mut chars = text.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().chain(chars).collect(),
        None => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn table_lists_every_rule_in_code_order() {
        let table = list(RulesFormat::Text);
        let codes: Vec<_> = table
            .lines()
            .skip(1)
            .filter_map(|line| line.split_whitespace().next())
            .collect();
        let expected: Vec<_> = metas().map(|meta| meta.code).collect();
        assert_eq!(codes, expected);
        assert!(table.starts_with("CODE    NAME "), "{table}");
    }

    #[test]
    fn json_is_an_array_of_rules() {
        let value: serde_json::Value =
            serde_json::from_str(&list(RulesFormat::Json)).expect("valid JSON");
        let rules = value.as_array().expect("an array");
        assert_eq!(rules.len(), rules::ALL.len());
        assert_eq!(rules[0]["code"], "KL0001");
        assert_eq!(rules[0]["default_enabled"], true);
    }

    #[test]
    fn explain_by_code_or_name() {
        let by_code = explain("KL1002").expect("a rule");
        assert_eq!(Some(by_code.clone()), explain("missing-kind"));
        assert!(
            by_code.starts_with("# KL1002 `missing-kind`\n\n"),
            "{by_code}"
        );
        assert_eq!(explain("KL1"), None);
    }
}
