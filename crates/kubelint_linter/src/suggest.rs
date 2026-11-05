//! "Did you mean" suggestions for misspelled rule codes, names and categories.

use crate::{Category, Rule, rules};

/// The candidate closest to `input`, if it is close enough to be a likely typo.
///
/// Comparison ignores ASCII case and uses the edit distance (insertions, deletions and
/// substitutions of one character). A candidate qualifies when at most one edit per three
/// characters of `input` (and at least one edit) separates them. Ties go to the earliest
/// candidate, so callers list candidates in a meaningful order.
///
/// ```
/// use kubelint_linter::did_you_mean;
///
/// let names = ["missing-kind", "missing-api-version"];
/// assert_eq!(did_you_mean("mising-kind", names), Some("missing-kind"));
/// assert_eq!(did_you_mean("something-else", names), None);
/// ```
pub fn did_you_mean<'a>(
    input: &str,
    candidates: impl IntoIterator<Item = &'a str>,
) -> Option<&'a str> {
    let input = input.to_ascii_lowercase();
    let len = input.chars().count();
    if len == 0 {
        return None;
    }
    let limit = (len / 3).max(1);
    let mut best: Option<(usize, &'a str)> = None;
    for candidate in candidates {
        let distance = edit_distance(&input, &candidate.to_ascii_lowercase());
        if distance <= limit && best.is_none_or(|(d, _)| distance < d) {
            best = Some((distance, candidate));
        }
    }
    best.map(|(_, candidate)| candidate)
}

/// The built-in rule code or name closest to `input`, for an unknown rule in
/// `kubelint rule` or a suppression comment.
///
/// ```
/// use kubelint_linter::suggest_rule;
///
/// assert_eq!(suggest_rule("kl1001"), Some("KL1001"));
/// assert_eq!(suggest_rule("missing-kinds"), Some("missing-kind"));
/// ```
pub fn suggest_rule(input: &str) -> Option<&'static str> {
    rule_suggestion(rules::ALL, input, false)
}

/// The code, name or (with `categories`) category of `registry` closest to `input`.
pub(crate) fn rule_suggestion(
    registry: &[&'static dyn Rule],
    input: &str,
    categories: bool,
) -> Option<&'static str> {
    let codes = registry.iter().map(|rule| rule.meta().code);
    let names = registry.iter().map(|rule| rule.meta().name);
    let categories = Category::ALL
        .into_iter()
        .filter(move |_| categories)
        .map(Category::as_str);
    did_you_mean(input, codes.chain(names).chain(categories))
}

/// The Levenshtein distance between `a` and `b`, counted in characters.
fn edit_distance(a: &str, b: &str) -> usize {
    let b: Vec<char> = b.chars().collect();
    // `row[j]` is the distance between the prefix of `a` seen so far and `b[..j]`.
    let mut row: Vec<usize> = (0..=b.len()).collect();
    for (i, ca) in a.chars().enumerate() {
        let mut diagonal = row[0];
        row[0] = i + 1;
        for (j, &cb) in b.iter().enumerate() {
            let substitution = diagonal + usize::from(ca != cb);
            diagonal = row[j + 1];
            row[j + 1] = substitution.min(row[j] + 1).min(diagonal + 1);
        }
    }
    row[b.len()]
}

#[cfg(test)]
mod tests {
    use super::{did_you_mean, edit_distance, suggest_rule};

    #[test]
    fn distances() {
        assert_eq!(edit_distance("", ""), 0);
        assert_eq!(edit_distance("abc", ""), 3);
        assert_eq!(edit_distance("", "ab"), 2);
        assert_eq!(edit_distance("kitten", "sitting"), 3);
        assert_eq!(edit_distance("flaw", "lawn"), 2);
        assert_eq!(edit_distance("é", "e"), 1);
    }

    #[test]
    fn suggestions() {
        let candidates = ["KL1001", "KL1002", "missing-kind", "security"];
        assert_eq!(did_you_mean("kl1001", candidates), Some("KL1001"));
        assert_eq!(did_you_mean("KL1003", candidates), Some("KL1001"));
        assert_eq!(did_you_mean("Security", candidates), Some("security"));
        assert_eq!(did_you_mean("securty", candidates), Some("security"));
        assert_eq!(
            did_you_mean("missingkind", candidates),
            Some("missing-kind")
        );
        // Too far from everything, or too short to guess.
        assert_eq!(did_you_mean("reliability", candidates), None);
        assert_eq!(did_you_mean("KL", candidates), None);
        assert_eq!(did_you_mean("", candidates), None);
        assert_eq!(did_you_mean("x", candidates), None);
    }

    #[test]
    fn suggests_built_in_rules_only() {
        assert_eq!(suggest_rule("KL0O01"), Some("KL0001"));
        assert_eq!(suggest_rule("invalid_yaml"), Some("invalid-yaml"));
        assert_eq!(suggest_rule("security"), None);
    }
}
