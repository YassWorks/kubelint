//! KL1001 `missing-api-version`.

use kubelint_diagnostics::Severity;

use super::{TypeField, check_type_field, entry_range, missing_field_range};
use crate::{Category, ObjectContext, Rule, RuleMeta};

/// Reports an object without an `apiVersion`, or with an empty one.
pub struct MissingApiVersion;

static META: RuleMeta = RuleMeta {
    code: "KL1001",
    name: "missing-api-version",
    category: Category::Correctness,
    default_severity: Severity::Error,
    default_enabled: true,
    summary: "Object has no `apiVersion`.",
    explanation: include_str!("missing_api_version.md"),
};

impl Rule for MissingApiVersion {
    fn meta(&self) -> &'static RuleMeta {
        &META
    }

    fn check_object(&self, cx: &mut ObjectContext<'_>) {
        let object = cx.object();
        let Some(problem) = check_type_field(object, "apiVersion") else {
            return;
        };
        let kind = object
            .kind()
            .and_then(|kind| kind.as_str())
            .filter(|kind| !kind.trim().is_empty());
        let subject = match kind {
            Some(kind) => format!("`{kind}` object"),
            None => "object".to_owned(),
        };
        let (range, message) = match problem {
            TypeField::Missing => (
                missing_field_range(object, "kind"),
                format!("{subject} has no `apiVersion`"),
            ),
            TypeField::Empty(entry) => (
                entry_range(entry),
                format!("{subject} has an empty `apiVersion`"),
            ),
        };
        let help = match kind.and_then(kubelint_k8s::stable_api_version) {
            Some(api_version) => format!("add `apiVersion: {api_version}`"),
            None if kind.is_some() => {
                "add the `apiVersion` of this kind; `kubectl api-resources` lists them".to_owned()
            }
            None => "add `apiVersion` and `kind`, such as `apiVersion: apps/v1` and \
                     `kind: Deployment`"
                .to_owned(),
        };
        cx.report(cx.diagnostic(range, message).with_help(help));
    }
}
