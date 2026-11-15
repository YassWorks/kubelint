use std::collections::BTreeMap;
use std::fmt;

use kubelint_k8s::GroupVersionKind;
use serde::Deserialize;

/// The version of the compact JSON form. `cargo xtask import-schemas` writes it into each
/// file, and a file with another version is rejected.
pub const FORMAT_VERSION: u32 = 1;

/// The API schema of one Kubernetes version.
#[derive(Debug)]
pub struct Schema {
    definitions: Vec<Definition>,
    /// Sorted by [`Root::key`].
    roots: Vec<Root>,
}

/// A named type such as `io.k8s.api.apps.v1.Deployment`.
#[derive(Debug)]
pub struct Definition {
    pub name: String,
    pub shape: Shape,
}

/// An index into the definitions of the [`Schema`] that returned it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DefinitionId(usize);

/// What a value must look like.
#[derive(Debug)]
pub enum Shape {
    /// Accepts any value: a schema with no type, or an object with neither `properties`
    /// nor `additionalProperties` (`RawExtension`).
    Any,
    Scalar(Scalar),
    Array(Box<Shape>),
    /// An object with `additionalProperties`: any key, each value of this shape.
    Map(Box<Shape>),
    /// An object with `properties`: only these keys.
    Object(Object),
    /// Accepts a value that matches any of the shapes (`IntOrString`, `Quantity`).
    OneOf(Vec<Shape>),
    Ref(DefinitionId),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scalar {
    Boolean,
    Integer,
    Number,
    String,
}

#[derive(Debug)]
pub struct Object {
    pub properties: BTreeMap<String, Shape>,
    pub required: Vec<String>,
}

/// A schema file that cannot be read. Schema files are generated, so this is a bug in
/// the importer or a hand edit, never a user error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SchemaError(String);

impl fmt::Display for SchemaError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for SchemaError {}

impl Schema {
    /// Reads a schema file in the compact JSON form.
    ///
    /// ```
    /// use kubelint_k8s::{GroupVersion, GroupVersionKind};
    /// use kubelint_schema::{Schema, Shape};
    ///
    /// let schema = Schema::from_json(
    ///     r#"{"format": 1,
    ///         "definitions": {"ConfigMap": {"type": "object", "properties": {}}},
    ///         "kinds": [{"group": "", "version": "v1", "kind": "ConfigMap",
    ///                    "definition": "ConfigMap"}]}"#,
    /// )
    /// .unwrap();
    /// let gvk = GroupVersionKind::new(GroupVersion::parse("v1").unwrap(), "ConfigMap");
    /// let id = schema.lookup(gvk).unwrap();
    /// assert!(matches!(schema.definition(id).shape, Shape::Object(_)));
    /// ```
    pub fn from_json(text: &str) -> Result<Self, SchemaError> {
        let invalid =
            |error: serde_json::Error| SchemaError(format!("invalid schema file: {error}"));
        let header: Header = serde_json::from_str(text).map_err(invalid)?;
        if header.format != FORMAT_VERSION {
            return Err(SchemaError(format!(
                "schema format {} is not the supported format {FORMAT_VERSION}",
                header.format
            )));
        }
        let raw: RawSchema = serde_json::from_str(text).map_err(invalid)?;
        let (names, shapes): (Vec<String>, Vec<RawShape>) = raw.definitions.into_iter().unzip();
        let shapes = shapes
            .into_iter()
            .zip(&names)
            .map(|(shape, name)| {
                shape
                    .convert(&names)
                    .map_err(|message| SchemaError(format!("definition `{name}`: {message}")))
            })
            .collect::<Result<Vec<_>, _>>()?;
        let mut roots = raw
            .kinds
            .into_iter()
            .map(|kind| {
                let definition = definition_id(&names, &kind.definition).map_err(|message| {
                    let key = (
                        kind.group.as_str(),
                        kind.version.as_str(),
                        kind.kind.as_str(),
                    );
                    SchemaError(format!("kind {}: {message}", Key(key)))
                })?;
                Ok(Root {
                    group: kind.group,
                    version: kind.version,
                    kind: kind.kind,
                    definition,
                })
            })
            .collect::<Result<Vec<_>, SchemaError>>()?;
        roots.sort_by(|a, b| a.key().cmp(&b.key()));
        if let Some(pair) = roots.windows(2).find(|pair| pair[0].key() == pair[1].key()) {
            return Err(SchemaError(format!(
                "kind {} is listed twice",
                Key(pair[0].key())
            )));
        }
        let definitions = names
            .into_iter()
            .zip(shapes)
            .map(|(name, shape)| Definition { name, shape })
            .collect();
        let schema = Self { definitions, roots };
        if let Some(definition) = schema
            .definitions
            .iter()
            .find(|d| schema.is_cycle(&d.shape))
        {
            return Err(SchemaError(format!(
                "definition `{}` refers to itself",
                definition.name
            )));
        }
        Ok(schema)
    }

    /// The root definition of a kind, `None` for a kind this version does not have.
    pub fn lookup(&self, gvk: GroupVersionKind<'_>) -> Option<DefinitionId> {
        let key = (gvk.group, gvk.version, gvk.kind);
        self.roots
            .binary_search_by(|root| root.key().cmp(&key))
            .ok()
            .map(|index| self.roots[index].definition)
    }

    /// # Panics
    ///
    /// When `id` comes from another schema with fewer definitions.
    pub fn definition(&self, id: DefinitionId) -> &Definition {
        &self.definitions[id.0]
    }

    /// Follows [`Shape::Ref`] until a shape that is not a reference.
    pub fn resolve<'a>(&'a self, mut shape: &'a Shape) -> &'a Shape {
        while let Shape::Ref(id) = shape {
            shape = &self.definition(*id).shape;
        }
        shape
    }

    /// Whether following references from `shape` never reaches a shape that is not one.
    fn is_cycle<'a>(&'a self, mut shape: &'a Shape) -> bool {
        for _ in 0..=self.definitions.len() {
            let Shape::Ref(id) = shape else {
                return false;
            };
            shape = &self.definition(*id).shape;
        }
        true
    }
}

#[derive(Debug)]
struct Root {
    group: String,
    version: String,
    kind: String,
    definition: DefinitionId,
}

impl Root {
    fn key(&self) -> (&str, &str, &str) {
        (&self.group, &self.version, &self.kind)
    }
}

/// A group, version and kind, written for messages.
struct Key<'a>((&'a str, &'a str, &'a str));

impl fmt::Display for Key<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let (group, version, kind) = self.0;
        write!(f, "`{group}` `{version}` `{kind}`")
    }
}

/// `names` is sorted, because it comes from the keys of a `BTreeMap`.
fn definition_id(names: &[String], name: &str) -> Result<DefinitionId, String> {
    names
        .binary_search_by(|candidate| candidate.as_str().cmp(name))
        .map(DefinitionId)
        .map_err(|_| format!("refers to the unknown definition `{name}`"))
}

#[derive(Deserialize)]
struct Header {
    format: u32,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawSchema {
    #[expect(dead_code, reason = "read and checked through `Header`")]
    format: u32,
    definitions: BTreeMap<String, RawShape>,
    kinds: Vec<RawKind>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawKind {
    group: String,
    version: String,
    kind: String,
    definition: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct RawShape {
    #[serde(rename = "type")]
    ty: Option<String>,
    properties: Option<BTreeMap<String, RawShape>>,
    required: Option<Vec<String>>,
    additional_properties: Option<Box<RawShape>>,
    items: Option<Box<RawShape>>,
    one_of: Option<Vec<RawShape>>,
    #[serde(rename = "$ref")]
    reference: Option<String>,
}

impl RawShape {
    fn convert(self, names: &[String]) -> Result<Shape, String> {
        if let Some(name) = self.reference {
            return definition_id(names, &name).map(Shape::Ref);
        }
        if let Some(shapes) = self.one_of {
            return shapes
                .into_iter()
                .map(|shape| shape.convert(names))
                .collect::<Result<_, _>>()
                .map(Shape::OneOf);
        }
        let Some(ty) = self.ty else {
            return Ok(Shape::Any);
        };
        Ok(match ty.as_str() {
            "boolean" => Shape::Scalar(Scalar::Boolean),
            "integer" => Shape::Scalar(Scalar::Integer),
            "number" => Shape::Scalar(Scalar::Number),
            "string" => Shape::Scalar(Scalar::String),
            "array" => {
                let items = self.items.ok_or("an array without `items`")?;
                Shape::Array(Box::new(items.convert(names)?))
            }
            "object" => convert_object(
                self.properties,
                self.required,
                self.additional_properties,
                names,
            )?,
            other => return Err(format!("unknown type `{other}`")),
        })
    }
}

fn convert_object(
    properties: Option<BTreeMap<String, RawShape>>,
    required: Option<Vec<String>>,
    additional_properties: Option<Box<RawShape>>,
    names: &[String],
) -> Result<Shape, String> {
    let Some(properties) = properties else {
        if required.is_some() {
            return Err("`required` without `properties`".to_owned());
        }
        return match additional_properties {
            Some(values) => Ok(Shape::Map(Box::new(values.convert(names)?))),
            None => Ok(Shape::Any),
        };
    };
    if additional_properties.is_some() {
        return Err("an object with both `properties` and `additionalProperties`".to_owned());
    }
    let required = required.unwrap_or_default();
    if let Some(missing) = required.iter().find(|name| !properties.contains_key(*name)) {
        return Err(format!("required property `{missing}` is not defined"));
    }
    let properties = properties
        .into_iter()
        .map(|(name, shape)| match shape.convert(names) {
            Ok(shape) => Ok((name, shape)),
            Err(message) => Err(format!("property `{name}`: {message}")),
        })
        .collect::<Result<_, String>>()?;
    Ok(Shape::Object(Object {
        properties,
        required,
    }))
}
