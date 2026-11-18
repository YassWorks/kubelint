use std::collections::BTreeMap;
use std::collections::btree_map::Entry;
use std::path::{Path, PathBuf};

use kubelint_schema::{FORMAT_VERSION, Schema};
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Relative to the repository root.
pub const SCHEMAS_DIR: &str = "crates/kubelint_schema/schemas";

const SOURCES_HEADER: &str = "# Written by `cargo xtask import-schemas`. Do not edit.\n";
const REF_PREFIX: &str = "#/components/schemas/";
const GROUP_VERSION_KIND: &str = "x-kubernetes-group-version-kind";
/// Keywords that validation does not use. Any keyword missing here and from [`shape`]
/// stops the import, so a new upstream keyword is never dropped unseen.
const DROPPED: [&str; 10] = [
    "default",
    "description",
    "format",
    GROUP_VERSION_KIND,
    "x-kubernetes-list-map-keys",
    "x-kubernetes-list-type",
    "x-kubernetes-map-type",
    "x-kubernetes-patch-merge-key",
    "x-kubernetes-patch-strategy",
    "x-kubernetes-unions",
];

/// The compact form of a schema, which `kubelint_schema` reads back.
#[derive(Debug, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
struct Shape {
    #[serde(rename = "$ref", skip_serializing_if = "Option::is_none")]
    reference: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    additional_properties: Option<Box<Shape>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    items: Option<Box<Shape>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    one_of: Option<Vec<Shape>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    properties: Option<BTreeMap<String, Shape>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    required: Option<Vec<String>>,
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    ty: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord)]
struct GroupVersionKind {
    group: String,
    version: String,
    kind: String,
}

#[derive(Serialize)]
struct KindEntry<'a> {
    #[serde(flatten)]
    kind: &'a GroupVersionKind,
    definition: &'a str,
}

#[derive(Deserialize)]
struct Document {
    #[serde(default)]
    components: Components,
}

#[derive(Deserialize, Default)]
struct Components {
    #[serde(default)]
    schemas: BTreeMap<String, Value>,
}

/// The compact schema file of one Kubernetes version, from its OpenAPI v3 documents
/// given as file name and content. A document that defines no kind is skipped.
pub fn import(documents: &[(String, String)]) -> Result<String, String> {
    let mut definitions: BTreeMap<&str, (Shape, &str)> = BTreeMap::new();
    let mut kinds: BTreeMap<GroupVersionKind, &str> = BTreeMap::new();
    let parsed = documents
        .iter()
        .map(|(file, text)| {
            serde_json::from_str::<Document>(text)
                .map(|document| (file.as_str(), document.components.schemas))
                .map_err(|error| format!("{file}: {error}"))
        })
        .collect::<Result<Vec<_>, _>>()?;
    for (file, schemas) in &parsed {
        if !schemas
            .values()
            .any(|schema| schema.get(GROUP_VERSION_KIND).is_some())
        {
            continue;
        }
        for (name, schema) in schemas {
            let in_definition = |error: String| format!("{file}: definition `{name}`: {error}");
            let shape = shape(schema).map_err(in_definition)?;
            match definitions.entry(name) {
                Entry::Vacant(entry) => {
                    entry.insert((shape, file));
                }
                Entry::Occupied(entry) if entry.get().0 != shape => {
                    return Err(format!(
                        "definition `{name}` differs between {} and {file}",
                        entry.get().1
                    ));
                }
                Entry::Occupied(_) => {}
            }
            let listed = schema
                .get(GROUP_VERSION_KIND)
                .map_or(Ok(Vec::new()), |value| {
                    Vec::<GroupVersionKind>::deserialize(value)
                        .map_err(|error| in_definition(format!("`{GROUP_VERSION_KIND}`: {error}")))
                })?;
            for kind in listed {
                match kinds.entry(kind) {
                    Entry::Vacant(entry) => {
                        entry.insert(name);
                    }
                    Entry::Occupied(entry) if *entry.get() != name => {
                        let GroupVersionKind {
                            group,
                            version,
                            kind,
                        } = entry.key();
                        return Err(format!(
                            "kind `{group}` `{version}` `{kind}` is defined by both `{}` and `{name}`",
                            entry.get()
                        ));
                    }
                    Entry::Occupied(_) => {}
                }
            }
        }
    }
    if kinds.is_empty() {
        return Err("no document defines a kind".to_owned());
    }
    let definitions = definitions
        .iter()
        .map(|(name, (shape, _))| format!("{}:{}", json(name), json(shape)))
        .collect::<Vec<_>>()
        .join(",\n");
    let kinds = kinds
        .iter()
        .map(|(kind, definition)| json(&KindEntry { kind, definition }))
        .collect::<Vec<_>>()
        .join(",\n");
    let text = format!(
        "{{\"format\":{FORMAT_VERSION},\n\"definitions\":{{\n{definitions}\n}},\n\"kinds\":[\n{kinds}\n]}}\n"
    );
    Schema::from_json(&text)
        .map_err(|error| format!("the imported schema does not load: {error}"))?;
    Ok(text)
}

fn json(value: &impl Serialize) -> String {
    serde_json::to_string(value).expect("a shape, kind or name always serializes")
}

fn shape(value: &Value) -> Result<Shape, String> {
    let Value::Object(schema) = value else {
        return Err("a schema that is not an object".to_owned());
    };
    for wrapper in ["allOf", "$ref"] {
        if !schema.contains_key(wrapper) {
            continue;
        }
        if let Some(other) = schema
            .keys()
            .find(|key| *key != wrapper && !DROPPED.contains(&key.as_str()))
        {
            return Err(format!("`{wrapper}` next to `{other}`"));
        }
    }
    if let Some(all_of) = schema.get("allOf") {
        return match all_of.as_array().map(Vec::as_slice) {
            Some([single]) => shape(single),
            _ => Err("an `allOf` that is not a single schema".to_owned()),
        };
    }
    let mut out = Shape::default();
    for (key, value) in schema {
        let within = |error: String| format!("`{key}`: {error}");
        match key.as_str() {
            "$ref" => {
                let name = value
                    .as_str()
                    .and_then(|reference| reference.strip_prefix(REF_PREFIX))
                    .ok_or_else(|| format!("`$ref` outside `{REF_PREFIX}`: {value}"))?;
                out.reference = Some(name.to_owned());
            }
            "type" => out.ty = Some(string(value).map_err(within)?),
            "required" => {
                let names = value.as_array().ok_or("`required` is not a list")?;
                out.required = Some(names.iter().map(string).collect::<Result<_, _>>()?);
            }
            "properties" => {
                let properties = value.as_object().ok_or("`properties` is not an object")?;
                let properties = properties.iter().map(|(name, value)| {
                    shape(value)
                        .map(|shape| (name.clone(), shape))
                        .map_err(|error| format!("property `{name}`: {error}"))
                });
                out.properties = Some(properties.collect::<Result<_, _>>()?);
            }
            "additionalProperties" => {
                out.additional_properties = Some(Box::new(shape(value).map_err(within)?));
            }
            "items" => out.items = Some(Box::new(shape(value).map_err(within)?)),
            "oneOf" => {
                let shapes = value.as_array().ok_or("`oneOf` is not a list")?;
                out.one_of = Some(
                    shapes
                        .iter()
                        .map(shape)
                        .collect::<Result<_, _>>()
                        .map_err(within)?,
                );
            }
            key if DROPPED.contains(&key) => {}
            other => return Err(format!("unsupported keyword `{other}`")),
        }
    }
    Ok(out)
}

fn string(value: &Value) -> Result<String, String> {
    value
        .as_str()
        .map(str::to_owned)
        .ok_or_else(|| format!("{value} is not a string"))
}

/// The minor version (`1.37`) of a release tag (`v1.37.2`), `None` for any other tag,
/// pre-releases included.
pub fn minor_version(tag: &str) -> Option<&str> {
    let version = tag.strip_prefix('v')?;
    let parts: Vec<&str> = version.split('.').collect();
    let [major, minor, patch] = parts.as_slice() else {
        return None;
    };
    let number = |part: &str| !part.is_empty() && part.bytes().all(|byte| byte.is_ascii_digit());
    (number(major) && number(minor) && number(patch))
        .then(|| &version[..major.len() + 1 + minor.len()])
}

/// Where the schema of one minor version was imported from.
#[derive(Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Source {
    pub tag: String,
    pub commit: String,
}

/// The `sources.toml` entries, keyed by minor version.
pub fn read_sources(text: &str) -> Result<BTreeMap<String, Source>, toml::de::Error> {
    toml::from_str(text)
}

pub fn write_sources(sources: &BTreeMap<String, Source>) -> String {
    let body = toml::to_string(sources).expect("a map of sources always serializes");
    format!("{SOURCES_HEADER}{body}")
}

/// The `.json` files of `dir`, sorted.
pub fn json_files(dir: &Path) -> std::io::Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    for entry in std::fs::read_dir(dir)? {
        let path = entry?.path();
        if path
            .extension()
            .is_some_and(|extension| extension == "json")
        {
            files.push(path);
        }
    }
    files.sort();
    Ok(files)
}
