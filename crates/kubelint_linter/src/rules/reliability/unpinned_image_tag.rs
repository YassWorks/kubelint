//! KL3003 `unpinned-image-tag`.

use kubelint_diagnostics::Severity;
use kubelint_k8s::{Container, ImageRef};

use crate::{Category, ObjectContext, Rule, RuleMeta};

/// Reports a container image without a digest whose tag is missing or `latest`.
///
/// Kubernetes pulls `:latest` for an image without a tag (`ParseImageName` in
/// `pkg/util/parsers/parsers.go`). Images that are not well-formed references are
/// skipped: empty values, templates such as `{{ .Values.image }}` or `${IMAGE}`,
/// surrounding whitespace and uppercase repository paths, which the kubelet rejects.
pub struct UnpinnedImageTag;

static META: RuleMeta = RuleMeta {
    code: "KL3003",
    name: "unpinned-image-tag",
    category: Category::Reliability,
    default_severity: Severity::Warning,
    default_enabled: true,
    summary: "An image uses `:latest` or no tag at all (digests are fine).",
    explanation: include_str!("unpinned_image_tag.md"),
};

impl Rule for UnpinnedImageTag {
    fn meta(&self) -> &'static RuleMeta {
        &META
    }

    fn check_object(&self, cx: &mut ObjectContext<'_>) {
        let Some(pod) = cx.object().pod_spec() else {
            return;
        };
        for node in pod.containers().filter_map(Container::image) {
            let Some(image) = node.as_str().filter(|image| is_reference(image)) else {
                continue;
            };
            let reference = ImageRef::parse(image);
            if reference.path().bytes().any(|b| b.is_ascii_uppercase()) {
                continue;
            }
            let problem = match (reference.digest, reference.tag) {
                (None, None) => "has no tag, so it pulls `latest`",
                (None, Some("latest")) => "uses the `latest` tag",
                _ => continue,
            };
            cx.report(
                cx.diagnostic(node.range, format!("image `{image}` {problem}"))
                    .with_help(format!(
                        "pin a version tag, such as `{name}:1.2.3`, or a digest, such as \
                         `{name}@sha256:<digest>`",
                        name = reference.name,
                    )),
            );
        }
    }
}

/// Whether `image` is non-empty and uses only the characters of an image reference.
fn is_reference(image: &str) -> bool {
    !image.is_empty()
        && image
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"._-/:@[]".contains(&b))
}
