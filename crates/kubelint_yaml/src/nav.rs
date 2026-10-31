//! Read-only navigation over the tree.
//!
//! Every example parses a small document and takes its root, so the doctests double as
//! usage documentation.

use crate::{Entry, Node, NodeKind, Scalar};

impl Node {
    /// The scalar, if this node is one.
    ///
    /// ```
    /// let doc = &kubelint_yaml::parse("a: 1").documents[0];
    /// let value = doc.root.as_ref().unwrap().get("a").unwrap();
    /// assert_eq!(value.as_scalar().unwrap().value, "1");
    /// ```
    pub fn as_scalar(&self) -> Option<&Scalar> {
        match &self.kind {
            NodeKind::Scalar(s) => Some(s),
            _ => None,
        }
    }

    /// The scalar's text (quotes and escapes processed), if this node is a scalar.
    ///
    /// ```
    /// let doc = &kubelint_yaml::parse("a: \"x\"").documents[0];
    /// let root = doc.root.as_ref().unwrap();
    /// assert_eq!(root.get("a").and_then(|n| n.as_str()), Some("x"));
    /// assert_eq!(root.as_str(), None);
    /// ```
    pub fn as_str(&self) -> Option<&str> {
        self.as_scalar().map(|s| s.value.as_str())
    }

    /// The entries, in source order, if this node is a mapping.
    ///
    /// ```
    /// let doc = &kubelint_yaml::parse("a: 1\nb: 2").documents[0];
    /// assert_eq!(doc.root.as_ref().unwrap().as_mapping().unwrap().len(), 2);
    /// ```
    pub fn as_mapping(&self) -> Option<&[Entry]> {
        match &self.kind {
            NodeKind::Mapping(entries) => Some(entries),
            _ => None,
        }
    }

    /// The items, in source order, if this node is a sequence.
    ///
    /// ```
    /// let doc = &kubelint_yaml::parse("- a\n- b").documents[0];
    /// assert_eq!(doc.root.as_ref().unwrap().as_sequence().unwrap().len(), 2);
    /// ```
    pub fn as_sequence(&self) -> Option<&[Node]> {
        match &self.kind {
            NodeKind::Sequence(items) => Some(items),
            _ => None,
        }
    }

    /// Whether this is a missing value, as in `key:`.
    ///
    /// ```
    /// let doc = &kubelint_yaml::parse("a:").documents[0];
    /// assert!(doc.root.as_ref().unwrap().get("a").unwrap().is_null());
    /// ```
    pub fn is_null(&self) -> bool {
        matches!(self.kind, NodeKind::Null)
    }

    /// Iterates over the entries of a mapping. Empty for any other node.
    ///
    /// ```
    /// let doc = &kubelint_yaml::parse("a: 1\nb: 2").documents[0];
    /// let keys: Vec<_> = doc.root.as_ref().unwrap().entries().filter_map(|e| e.key_str()).collect();
    /// assert_eq!(keys, ["a", "b"]);
    /// ```
    pub fn entries(&self) -> impl Iterator<Item = &Entry> {
        self.as_mapping().unwrap_or_default().iter()
    }

    /// Iterates over the items of a sequence. Empty for any other node.
    ///
    /// ```
    /// let doc = &kubelint_yaml::parse("- a\n- b").documents[0];
    /// assert_eq!(doc.root.as_ref().unwrap().items().count(), 2);
    /// ```
    pub fn items(&self) -> impl Iterator<Item = &Node> {
        self.as_sequence().unwrap_or_default().iter()
    }

    /// The entry whose key is the string `key`, if this node is a mapping that has one.
    ///
    /// With duplicate keys the last one wins, as in Kubernetes' decoder. Use
    /// [`entries`](Self::entries) to see all of them. Use this when a diagnostic should
    /// point at the key.
    ///
    /// ```
    /// let doc = &kubelint_yaml::parse("é: 1\nname: x").documents[0];
    /// let entry = doc.root.as_ref().unwrap().get_entry("name").unwrap();
    /// assert_eq!(u32::from(entry.key.range.start()), 6);
    /// ```
    pub fn get_entry(&self, key: &str) -> Option<&Entry> {
        self.as_mapping()
            .unwrap_or_default()
            .iter()
            .rev()
            .find(|e| e.key_str() == Some(key))
    }

    /// The value for `key`, if this node is a mapping that has one. See
    /// [`get_entry`](Self::get_entry) for duplicates.
    ///
    /// ```
    /// let doc = &kubelint_yaml::parse("a: 1\na: 2").documents[0];
    /// let root = doc.root.as_ref().unwrap();
    /// assert_eq!(root.get("a").and_then(|n| n.as_str()), Some("2"));
    /// assert!(root.get("missing").is_none());
    /// ```
    pub fn get(&self, key: &str) -> Option<&Node> {
        self.get_entry(key).map(|e| &e.value)
    }

    /// Follows a path of mapping keys. An empty path returns this node.
    ///
    /// ```
    /// let doc = &kubelint_yaml::parse("spec:\n  template:\n    name: x").documents[0];
    /// let root = doc.root.as_ref().unwrap();
    /// let node = root.get_path(&["spec", "template", "name"]).unwrap();
    /// assert_eq!(node.as_str(), Some("x"));
    /// assert!(root.get_path(&["spec", "nope", "name"]).is_none());
    /// ```
    pub fn get_path(&self, path: &[&str]) -> Option<&Node> {
        path.iter().try_fold(self, |node, key| node.get(key))
    }
}

impl Entry {
    /// The key's text if the key is a scalar (it nearly always is).
    ///
    /// ```
    /// let doc = &kubelint_yaml::parse("a: 1").documents[0];
    /// let entry = &doc.root.as_ref().unwrap().as_mapping().unwrap()[0];
    /// assert_eq!(entry.key_str(), Some("a"));
    /// ```
    pub fn key_str(&self) -> Option<&str> {
        self.key.as_str()
    }
}

#[cfg(test)]
mod tests {
    use crate::parse;

    #[test]
    fn navigation_on_wrong_kind_is_empty() {
        let parse = parse("- a\n");
        let root = parse.documents[0].root.as_ref().unwrap();
        assert!(root.get("a").is_none());
        assert!(root.get_path(&["a"]).is_none());
        assert_eq!(root.entries().count(), 0);
        assert_eq!(root.get_path(&[]).map(|n| n.range), Some(root.range));
    }

    #[test]
    fn scalar_has_no_children() {
        let parse = parse("a: b\n");
        let value = parse.documents[0].root.as_ref().unwrap().get("a").unwrap();
        assert_eq!(value.items().count(), 0);
        assert!(value.as_mapping().is_none() && value.as_sequence().is_none());
    }
}
