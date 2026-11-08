use kubelint_source::TextRange;
use kubelint_yaml::{Document, Node};

use crate::{BuiltinKind, GroupVersion, GroupVersionKind};

/// A read-only view of one Kubernetes object in a YAML tree.
///
/// Accessors return the YAML [`Node`] of a field, not a converted value, so a diagnostic
/// can point at it with `node.range`. A field that is missing returns `None`; a field
/// with the wrong shape (a sequence where a string belongs) is returned as is, and rules
/// decide what to do with it.
#[derive(Debug, Clone, Copy)]
pub struct Object<'a> {
    node: &'a Node,
}

/// The Kubernetes objects of a document, in source order.
///
/// A document is an object when its root is a mapping with an `apiVersion` or a `kind`
/// key. Anything else (a GitHub workflow, a Helm `values.yaml`, an empty document) yields
/// nothing, which is what lets kubelint run on a whole repository without configuration.
///
/// A `kind: List` document yields the mappings in its `items` instead of itself, as
/// `kubectl` does. Every mapping item counts, with or without `apiVersion` and `kind`,
/// because inside a list it can only be meant as an object.
///
/// ```
/// let parse = kubelint_yaml::parse(
///     "apiVersion: v1\nkind: List\nitems:\n- kind: Pod\n- kind: Service\n",
/// );
/// let kinds: Vec<_> = kubelint_k8s::objects(&parse.documents[0])
///     .iter()
///     .filter_map(|o| o.kind().and_then(|k| k.as_str()))
///     .collect();
/// assert_eq!(kinds, ["Pod", "Service"]);
/// ```
pub fn objects(document: &Document) -> Vec<Object<'_>> {
    let Some(object) = document.root.as_ref().and_then(Object::from_node) else {
        return Vec::new();
    };
    if !object.is_list() {
        return vec![object];
    }
    object
        .node
        .get("items")
        .into_iter()
        .flat_map(Node::items)
        .filter(|item| item.as_mapping().is_some())
        .map(|node| Object { node })
        .collect()
}

impl<'a> Object<'a> {
    /// Views `node` as an object if it is a mapping with an `apiVersion` or `kind` key.
    /// The key is enough: `kind:` with no value is still an object (with a broken kind).
    ///
    /// ```
    /// use kubelint_k8s::Object;
    ///
    /// let parse = kubelint_yaml::parse("kind: Pod\n---\nname: ci\n");
    /// let root = |i: usize| parse.documents[i].root.as_ref().unwrap();
    /// assert!(Object::from_node(root(0)).is_some());
    /// assert!(Object::from_node(root(1)).is_none());
    /// ```
    pub fn from_node(node: &'a Node) -> Option<Self> {
        let is_object = node.as_mapping().is_some()
            && (node.get_entry("apiVersion").is_some() || node.get_entry("kind").is_some());
        is_object.then_some(Self { node })
    }

    pub fn node(self) -> &'a Node {
        self.node
    }

    pub fn range(self) -> TextRange {
        self.node.range
    }

    pub fn api_version(self) -> Option<&'a Node> {
        self.node.get("apiVersion")
    }

    pub fn kind(self) -> Option<&'a Node> {
        self.node.get("kind")
    }

    /// The parsed `apiVersion`, if it is a string Kubernetes would accept.
    ///
    /// ```
    /// let parse = kubelint_yaml::parse("apiVersion: apps/v1\nkind: Deployment\n");
    /// let object = kubelint_k8s::objects(&parse.documents[0])[0];
    /// assert_eq!(object.group_version().unwrap().group, "apps");
    /// ```
    pub fn group_version(self) -> Option<GroupVersion<'a>> {
        GroupVersion::parse(self.api_version()?.as_str()?)
    }

    /// The group, version and kind, if both `apiVersion` and `kind` are valid strings.
    ///
    /// ```
    /// let parse = kubelint_yaml::parse("apiVersion: v1\nkind: Pod\n");
    /// let gvk = kubelint_k8s::objects(&parse.documents[0])[0].gvk().unwrap();
    /// assert_eq!((gvk.group, gvk.version, gvk.kind), ("", "v1", "Pod"));
    /// ```
    pub fn gvk(self) -> Option<GroupVersionKind<'a>> {
        let kind = self.kind()?.as_str().filter(|k| !k.is_empty())?;
        Some(GroupVersionKind::new(self.group_version()?, kind))
    }

    /// The built-in kind, unless `apiVersion` names another group (a custom resource
    /// called `StatefulSet` is not a StatefulSet). Without a usable `apiVersion` the kind
    /// alone decides.
    ///
    /// ```
    /// use kubelint_k8s::BuiltinKind;
    ///
    /// let kind = |text: &str| {
    ///     kubelint_k8s::objects(&kubelint_yaml::parse(text).documents[0])[0].builtin_kind()
    /// };
    /// let statefulset = Some(BuiltinKind::StatefulSet);
    /// assert_eq!(kind("apiVersion: apps/v1\nkind: StatefulSet\n"), statefulset);
    /// assert_eq!(kind("kind: StatefulSet\n"), statefulset);
    /// assert_eq!(kind("apiVersion: example.com/v1\nkind: StatefulSet\n"), None);
    /// ```
    pub fn builtin_kind(self) -> Option<BuiltinKind> {
        let kind = BuiltinKind::parse(self.kind()?.as_str()?)?;
        self.group_version()
            .is_none_or(|gv| gv.group == kind.group())
            .then_some(kind)
    }

    pub fn metadata(self) -> Option<&'a Node> {
        self.node.get("metadata")
    }

    /// The `metadata.name` value.
    ///
    /// ```
    /// let parse = kubelint_yaml::parse("kind: Pod\nmetadata:\n  name: web\n");
    /// let object = kubelint_k8s::objects(&parse.documents[0])[0];
    /// assert_eq!(object.name().and_then(|n| n.as_str()), Some("web"));
    /// ```
    pub fn name(self) -> Option<&'a Node> {
        self.metadata()?.get("name")
    }

    /// The `metadata.namespace` value.
    pub fn namespace(self) -> Option<&'a Node> {
        self.metadata()?.get("namespace")
    }

    /// The `metadata.labels` value (normally a mapping of strings).
    pub fn labels(self) -> Option<&'a Node> {
        self.metadata()?.get("labels")
    }

    /// The `metadata.annotations` value (normally a mapping of strings).
    pub fn annotations(self) -> Option<&'a Node> {
        self.metadata()?.get("annotations")
    }

    fn is_list(self) -> bool {
        self.kind().and_then(Node::as_str) == Some("List")
    }
}
