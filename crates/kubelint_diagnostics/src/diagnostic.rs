use kubelint_source::{FileId, TextRange};

/// How serious a finding is. Ordered `Info < Warning < Error`, so `severity >= fail_on`
/// decides whether a run fails.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Severity {
    /// Advisory. Never fails a run unless the threshold is `info`.
    Info,
    /// Likely a problem. Fails a run by default.
    Warning,
    /// Definitely a problem.
    Error,
}

impl Severity {
    /// The lowercase name used in config files, the command line and output.
    ///
    /// ```
    /// use kubelint_diagnostics::Severity;
    ///
    /// assert_eq!(Severity::Warning.as_str(), "warning");
    /// assert!(Severity::Error > Severity::Warning);
    /// ```
    pub fn as_str(self) -> &'static str {
        match self {
            Severity::Info => "info",
            Severity::Warning => "warning",
            Severity::Error => "error",
        }
    }
}

/// A highlighted range of a file with an optional short note, shown under the snippet.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Label {
    pub file: FileId,
    pub range: TextRange,
    pub message: Option<String>,
}

impl Label {
    pub fn new(file: FileId, range: TextRange) -> Self {
        Self {
            file,
            range,
            message: None,
        }
    }

    #[must_use]
    pub fn with_message(mut self, message: impl Into<String>) -> Self {
        self.message = Some(message.into());
        self
    }
}

/// A text replacement. Inserting is an empty `range`, deleting is an empty `replacement`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Edit {
    pub range: TextRange,
    pub replacement: String,
}

/// How much a user can trust a [`Fix`] to keep the manifest's meaning.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Applicability {
    /// Never changes behavior. Applied by `--fix`.
    Safe,
    /// May change behavior. Applied only with `--unsafe-fixes`.
    Unsafe,
}

/// A set of edits that resolves a diagnostic. All edits apply to the file of the
/// diagnostic's primary label.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Fix {
    pub applicability: Applicability,
    /// The edits, which must not overlap.
    pub edits: Vec<Edit>,
}

/// One finding: which rule, how serious, what is wrong and where.
///
/// ```
/// use kubelint_diagnostics::{Diagnostic, Label, Severity};
/// use kubelint_source::{FileId, TextRange};
///
/// let range = TextRange::new(10.into(), 14.into());
/// let d = Diagnostic::new(
///     "KL2001",
///     Severity::Error,
///     "container runs in privileged mode",
///     Label::new(FileId(0), range).with_message("set to true here"),
/// )
/// .with_help("remove `privileged: true` or set it to false");
/// assert_eq!(d.code, "KL2001");
/// assert!(d.secondary.is_empty());
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
    /// The rule code, such as `KL2001`.
    pub code: &'static str,
    /// How serious the finding is, after config overrides.
    pub severity: Severity,
    /// What is wrong, as one sentence.
    pub message: String,
    pub primary: Label,
    /// Related locations, such as the other half of a duplicate.
    pub secondary: Vec<Label>,
    /// How to fix it, in plain words.
    pub help: Option<String>,
    pub fix: Option<Fix>,
}

impl Diagnostic {
    pub fn new(
        code: &'static str,
        severity: Severity,
        message: impl Into<String>,
        primary: Label,
    ) -> Self {
        Self {
            code,
            severity,
            message: message.into(),
            primary,
            secondary: Vec::new(),
            help: None,
            fix: None,
        }
    }

    #[must_use]
    pub fn with_secondary(mut self, label: Label) -> Self {
        self.secondary.push(label);
        self
    }

    #[must_use]
    pub fn with_help(mut self, help: impl Into<String>) -> Self {
        self.help = Some(help.into());
        self
    }

    #[must_use]
    pub fn with_fix(mut self, fix: Fix) -> Self {
        self.fix = Some(fix);
        self
    }
}
