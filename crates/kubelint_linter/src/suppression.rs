//! Suppression comments (`docs/user/configuration.md`).
//!
//! ```yaml
//! securityContext:
//!   # kubelint-disable-next-line KL2001 -- the CNI plugin needs this
//!   privileged: true
//!   allowPrivilegeEscalation: true  # kubelint-disable-line privilege-escalation-allowed
//! # kubelint-disable-file KL3001, missing-resource-requests
//! ```
//!
//! A diagnostic is suppressed when its primary label starts on a suppressed line, or when
//! its rule is disabled for the whole file. Rules are named by code or name, separated by
//! commas; an optional `-- reason` follows. KL0001 `invalid-yaml` cannot be suppressed,
//! like it cannot be ignored in configuration. Rules that cannot be resolved
//! silence nothing; KL0003 `invalid-suppression` reports them.

use std::collections::HashMap;

use kubelint_diagnostics::Diagnostic;
use kubelint_source::{SourceFile, TextRange, TextSize};
use kubelint_yaml::Comment;

use crate::rules::syntax::InvalidYaml;
use crate::{Rule as _, rules};

/// Which lines a directive applies to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DirectiveKind {
    /// `kubelint-disable-next-line`: the next line that is not only a comment.
    NextLine,
    /// `kubelint-disable-line`: the comment's own line.
    Line,
    /// `kubelint-disable-file`: the whole file.
    File,
}

impl DirectiveKind {
    const ALL: [(Self, &'static str); 3] = [
        (Self::NextLine, "kubelint-disable-next-line"),
        (Self::Line, "kubelint-disable-line"),
        (Self::File, "kubelint-disable-file"),
    ];
}

/// One parsed `# kubelint-disable-...` comment.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Directive {
    pub kind: DirectiveKind,
    /// The rule codes or names as written, with their ranges in the file.
    pub rules: Vec<(String, TextRange)>,
    /// The text after `--`, if any.
    pub reason: Option<String>,
}

/// Parses a comment as a suppression directive. `None` when it is an ordinary comment.
///
/// The directive must be the first thing in the comment (`# kubelint-disable-line X`),
/// so a comment that merely mentions it is not one.
pub(crate) fn parse_directive(comment: &Comment) -> Option<Directive> {
    let text = comment.text.as_str();
    let body = text.trim_start();
    // Offset of `body` in the file: after the `#` and the leading spaces.
    let body_start = usize::from(comment.range.start()) + 1 + (text.len() - body.len());
    let (kind, rest) = DirectiveKind::ALL.into_iter().find_map(|(kind, keyword)| {
        let rest = body.strip_prefix(keyword)?;
        let at_boundary = rest.is_empty() || rest.starts_with(char::is_whitespace);
        at_boundary.then_some((kind, rest))
    })?;
    let rest_start = body_start + (body.len() - rest.len());
    let (list, reason) = match rest.find("--") {
        Some(i) => (&rest[..i], Some(rest[i + 2..].trim().to_owned())),
        None => (rest, None),
    };
    let mut rules = Vec::new();
    let mut offset = rest_start;
    for part in list.split(',') {
        let trimmed = part.trim();
        if !trimmed.is_empty() {
            let start = offset + (part.len() - part.trim_start().len());
            rules.push((trimmed.to_owned(), range(start, start + trimmed.len())));
        }
        offset += part.len() + 1;
    }
    Some(Directive {
        kind,
        rules,
        reason: reason.filter(|r| !r.is_empty()),
    })
}

fn range(start: usize, end: usize) -> TextRange {
    let size = |n: usize| TextSize::new(u32::try_from(n).unwrap_or(u32::MAX));
    TextRange::new(size(start), size(end))
}

/// What a rule named in a suppression comment refers to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Target {
    /// A built-in rule (by code or name), with its code.
    Rule(&'static str),
    /// KL0001 `invalid-yaml`, which cannot be suppressed.
    AlwaysOn,
    /// Not a code or name of any built-in rule.
    Unknown,
}

/// Resolves a rule named in a suppression comment. Any built-in rule can be named, even
/// one that is not enabled; codes and names only, not categories or prefixes.
pub(crate) fn target(selector: &str) -> Target {
    let always_on = InvalidYaml.meta().code;
    match rules::ALL
        .iter()
        .map(|rule| rule.meta())
        .find(|meta| meta.code == selector || meta.name == selector)
    {
        Some(meta) if meta.code == always_on => Target::AlwaysOn,
        Some(meta) => Target::Rule(meta.code),
        None => Target::Unknown,
    }
}

/// The suppressions of one file.
#[derive(Debug, Default)]
pub(crate) struct Suppressions {
    /// Rule codes suppressed on each 1-based line.
    lines: HashMap<u32, Vec<&'static str>>,
    /// Rule codes suppressed in the whole file.
    file: Vec<&'static str>,
}

impl Suppressions {
    /// Reads the directives among `comments`, the comments of `file`.
    pub(crate) fn new(file: &SourceFile, comments: &[Comment]) -> Self {
        let index = file.line_index();
        let text = file.text();
        let line_of = |offset: TextSize| index.line_col(text, offset).line;
        // Lines holding nothing but a comment, which `next-line` skips over.
        let comment_lines: Vec<u32> = comments
            .iter()
            .filter(|c| {
                let line = line_of(c.range.start());
                index.line_range(text, line).is_some_and(|r| {
                    let before = &text[usize::from(r.start())..usize::from(c.range.start())];
                    before.trim().is_empty()
                })
            })
            .map(|c| line_of(c.range.start()))
            .collect();
        let mut suppressions = Self::default();
        for comment in comments {
            let Some(directive) = parse_directive(comment) else {
                continue;
            };
            let codes = directive
                .rules
                .iter()
                .filter_map(|(selector, _)| match target(selector) {
                    Target::Rule(code) => Some(code),
                    Target::AlwaysOn | Target::Unknown => None,
                });
            let line = line_of(comment.range.start());
            match directive.kind {
                DirectiveKind::File => suppressions.file.extend(codes),
                DirectiveKind::Line => suppressions.lines.entry(line).or_default().extend(codes),
                DirectiveKind::NextLine => {
                    let mut next = line + 1;
                    while comment_lines.contains(&next) {
                        next += 1;
                    }
                    suppressions.lines.entry(next).or_default().extend(codes);
                }
            }
        }
        suppressions
    }

    /// Whether `diagnostic`, found in `file`, is suppressed.
    pub(crate) fn suppresses(&self, file: &SourceFile, diagnostic: &Diagnostic) -> bool {
        let code = diagnostic.code;
        if self.file.contains(&code) {
            return true;
        }
        let line = file
            .line_index()
            .line_col(file.text(), diagnostic.primary.range.start())
            .line;
        self.lines
            .get(&line)
            .is_some_and(|codes| codes.contains(&code))
    }
}

#[cfg(test)]
mod tests {
    use kubelint_diagnostics::{Label, Severity};
    use kubelint_source::FileId;

    use super::*;

    fn directive(text: &str) -> Option<Directive> {
        let parse = kubelint_yaml::parse(text);
        parse.comments.first().and_then(parse_directive)
    }

    #[test]
    fn parses_rules_reason_and_positions() {
        let text = "a: 1  # kubelint-disable-line KL1001,  missing-kind -- generated é\n";
        let directive = directive(text).expect("a directive");
        assert_eq!(directive.kind, DirectiveKind::Line);
        let rules: Vec<_> = directive
            .rules
            .iter()
            .map(|(rule, r)| {
                let at = &text[usize::from(r.start())..usize::from(r.end())];
                (rule.as_str(), at)
            })
            .collect();
        assert_eq!(
            rules,
            [("KL1001", "KL1001"), ("missing-kind", "missing-kind")]
        );
        assert_eq!(directive.reason.as_deref(), Some("generated é"));
    }

    #[test]
    fn ordinary_comments_are_not_directives() {
        assert_eq!(directive("# just a note\n"), None);
        assert_eq!(directive("# see kubelint-disable-line KL1001\n"), None);
        assert_eq!(directive("# kubelint-disable-lines KL1001\n"), None);
        let bare = directive("#kubelint-disable-file\n").expect("a directive");
        assert_eq!((bare.kind, bare.rules.len()), (DirectiveKind::File, 0));
    }

    fn diagnostic(code: &'static str, file: &SourceFile, line: u32) -> Diagnostic {
        let start = file.line_index().offset(line, 1).expect("the line exists");
        Diagnostic::new(
            code,
            Severity::Error,
            "x",
            Label::new(FileId(0), TextRange::empty(start)),
        )
    }

    #[test]
    fn next_line_skips_comment_lines() {
        let text = "\
# kubelint-disable-next-line KL1001
# another comment
a: 1
b: 2 # kubelint-disable-line missing-kind
c: 3
";
        let file = SourceFile::new("f.yaml", text);
        let suppressions = Suppressions::new(&file, &kubelint_yaml::parse(text).comments);
        let suppressed =
            |code, line| suppressions.suppresses(&file, &diagnostic(code, &file, line));
        assert!(suppressed("KL1001", 3));
        assert!(!suppressed("KL1002", 3));
        assert!(suppressed("KL1002", 4));
        assert!(!suppressed("KL1001", 4));
        assert!(!suppressed("KL1001", 5));
    }

    #[test]
    fn file_directives_cover_every_line_but_not_invalid_yaml() {
        let text = "a: 1\n# kubelint-disable-file KL1001, invalid-yaml, KL0001, nope\n";
        let file = SourceFile::new("f.yaml", text);
        let suppressions = Suppressions::new(&file, &kubelint_yaml::parse(text).comments);
        assert!(suppressions.suppresses(&file, &diagnostic("KL1001", &file, 1)));
        assert!(!suppressions.suppresses(&file, &diagnostic("KL0001", &file, 1)));
    }
}
