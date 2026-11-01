//! What a rule sees while it runs, and how it reports findings.

use kubelint_diagnostics::{Diagnostic, Label, Severity};
use kubelint_k8s::Object;
use kubelint_source::{FileId, SourceFile, TextRange};

use crate::RuleMeta;

/// One Kubernetes object of the run, with the file it came from.
#[derive(Debug, Clone, Copy)]
pub struct LintObject<'a> {
    /// The file the object is in.
    pub file: FileId,
    /// The object.
    pub object: Object<'a>,
}

/// Builds diagnostics with the running rule's code and configured severity.
struct Reporter<'a> {
    meta: &'static RuleMeta,
    severity: Severity,
    diagnostics: &'a mut Vec<Diagnostic>,
}

impl Reporter<'_> {
    fn diagnostic(&self, file: FileId, range: TextRange, message: String) -> Diagnostic {
        Diagnostic::new(
            self.meta.code,
            self.severity,
            message,
            Label::new(file, range),
        )
    }
}

/// The context of [`Rule::check_object`](crate::Rule::check_object): one object and the
/// file it is in.
pub struct ObjectContext<'a> {
    file_id: FileId,
    file: &'a SourceFile,
    object: Object<'a>,
    reporter: Reporter<'a>,
}

impl<'a> ObjectContext<'a> {
    pub(crate) fn new(
        file_id: FileId,
        file: &'a SourceFile,
        object: Object<'a>,
        meta: &'static RuleMeta,
        severity: Severity,
        diagnostics: &'a mut Vec<Diagnostic>,
    ) -> Self {
        Self {
            file_id,
            file,
            object,
            reporter: Reporter {
                meta,
                severity,
                diagnostics,
            },
        }
    }

    /// The object being checked.
    pub fn object(&self) -> Object<'a> {
        self.object
    }

    /// The file the object is in.
    pub fn file(&self) -> &'a SourceFile {
        self.file
    }

    /// A diagnostic for the running rule pointing at `range` in this object's file, with
    /// the rule's code and configured severity. Add help or labels, then pass it to
    /// [`report`](Self::report).
    pub fn diagnostic(&self, range: TextRange, message: impl Into<String>) -> Diagnostic {
        self.reporter
            .diagnostic(self.file_id, range, message.into())
    }

    /// A label for `range` in this object's file, for secondary locations.
    pub fn label(&self, range: TextRange) -> Label {
        Label::new(self.file_id, range)
    }

    /// Records a finding.
    pub fn report(&mut self, diagnostic: Diagnostic) {
        self.reporter.diagnostics.push(diagnostic);
    }
}

/// The context of [`Rule::check_all`](crate::Rule::check_all): every object of the run.
pub struct RunContext<'a> {
    files: &'a [SourceFile],
    objects: &'a [LintObject<'a>],
    reporter: Reporter<'a>,
}

impl<'a> RunContext<'a> {
    pub(crate) fn new(
        files: &'a [SourceFile],
        objects: &'a [LintObject<'a>],
        meta: &'static RuleMeta,
        severity: Severity,
        diagnostics: &'a mut Vec<Diagnostic>,
    ) -> Self {
        Self {
            files,
            objects,
            reporter: Reporter {
                meta,
                severity,
                diagnostics,
            },
        }
    }

    /// Every object of the run, ordered by file and then by position.
    pub fn objects(&self) -> &'a [LintObject<'a>] {
        self.objects
    }

    /// The file with this id, if it is part of the run.
    pub fn file(&self, id: FileId) -> Option<&'a SourceFile> {
        self.files.get(usize::try_from(id.0).ok()?)
    }

    /// A diagnostic for the running rule pointing at `range` in `file`. See
    /// [`ObjectContext::diagnostic`].
    pub fn diagnostic(
        &self,
        file: FileId,
        range: TextRange,
        message: impl Into<String>,
    ) -> Diagnostic {
        self.reporter.diagnostic(file, range, message.into())
    }

    /// Records a finding.
    pub fn report(&mut self, diagnostic: Diagnostic) {
        self.reporter.diagnostics.push(diagnostic);
    }
}
