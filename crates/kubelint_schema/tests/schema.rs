use kubelint_k8s::{GroupVersion, GroupVersionKind};
use kubelint_schema::{Scalar, Schema, Shape};

const APPS: &str = include_str!("../resources/test/fixtures/apps.json");

fn schema() -> Schema {
    Schema::from_json(APPS).expect("the fixture is a valid schema")
}

fn gvk<'a>(api_version: &'a str, kind: &'a str) -> GroupVersionKind<'a> {
    GroupVersionKind::new(
        GroupVersion::parse(api_version).expect("tests use valid API versions"),
        kind,
    )
}

/// The shape of `path` (property names, `[]` for array items) inside a kind's root.
fn shape_at<'a>(schema: &'a Schema, root: &'a Shape, path: &[&str]) -> &'a Shape {
    path.iter().fold(schema.resolve(root), |shape, step| {
        let next = match (shape, *step) {
            (Shape::Array(items), "[]") => &**items,
            (Shape::Object(object), name) => &object.properties[name],
            (other, step) => panic!("cannot take `{step}` of {other:?}"),
        };
        schema.resolve(next)
    })
}

#[test]
fn looks_up_kinds_by_group_version_and_kind() {
    let schema = schema();
    let deployment = schema.lookup(gvk("apps/v1", "Deployment")).unwrap();
    assert_eq!(
        schema.definition(deployment).name,
        "io.k8s.api.apps.v1.Deployment"
    );
    let core = schema.lookup(gvk("v1", "DeleteOptions")).unwrap();
    let apps = schema.lookup(gvk("apps/v1", "DeleteOptions")).unwrap();
    assert_eq!(core, apps);
}

#[test]
fn unknown_kinds_are_not_found() {
    let schema = schema();
    for (api_version, kind) in [
        ("apps/v1beta1", "Deployment"),
        ("v1", "Deployment"),
        ("apps/v1", "deployment"),
        ("example.com/v1", "Widget"),
    ] {
        assert!(
            schema.lookup(gvk(api_version, kind)).is_none(),
            "{api_version} {kind}"
        );
    }
}

#[test]
fn reads_every_shape() {
    let schema = schema();
    let id = schema.lookup(gvk("apps/v1", "Deployment")).unwrap();
    let root = &schema.definition(id).shape;
    let at = |path: &[&str]| shape_at(&schema, root, path);

    assert!(matches!(
        at(&["spec", "replicas"]),
        Shape::Scalar(Scalar::Integer)
    ));
    assert!(matches!(
        at(&["spec", "paused"]),
        Shape::Scalar(Scalar::Boolean)
    ));
    assert!(matches!(
        at(&["spec", "containers", "[]", "cpu"]),
        Shape::Scalar(Scalar::Number)
    ));
    assert!(matches!(
        at(&["metadata", "name"]),
        Shape::Scalar(Scalar::String)
    ));
    assert!(matches!(
        at(&["metadata", "labels"]),
        Shape::Map(values) if matches!(**values, Shape::Scalar(Scalar::String))
    ));
    assert!(matches!(at(&["spec", "maxSurge"]), Shape::OneOf(shapes) if shapes.len() == 2));
    assert!(matches!(at(&["spec", "extension"]), Shape::Any));
    assert!(matches!(at(&["spec", "anything"]), Shape::Any));
}

#[test]
fn keeps_required_fields() {
    let schema = schema();
    let id = schema.lookup(gvk("apps/v1", "Deployment")).unwrap();
    let root = &schema.definition(id).shape;
    let Shape::Object(spec) = shape_at(&schema, root, &["spec"]) else {
        panic!("spec is not an object");
    };
    assert_eq!(spec.required, ["selector"]);
    let Shape::Object(deployment) = schema.resolve(root) else {
        panic!("Deployment is not an object");
    };
    assert!(deployment.required.is_empty());
}

const INVALID: &[(&str, &str)] = &[
    (
        r#"{"format": 2, "definitions": {}, "kinds": []}"#,
        "schema format 2 is not the supported format 1",
    ),
    (
        r#"{"format": 1, "definitions": {"A": {"$ref": "B"}}, "kinds": []}"#,
        "definition `A`: refers to the unknown definition `B`",
    ),
    (
        r#"{"format": 1, "definitions": {"A": {"type": "map"}}, "kinds": []}"#,
        "definition `A`: unknown type `map`",
    ),
    (
        r#"{"format": 1, "definitions": {"A": {"type": "array"}}, "kinds": []}"#,
        "definition `A`: an array without `items`",
    ),
    (
        r#"{"format": 1, "definitions": {"A": {"type": "object", "properties": {"x": {"type": "date"}}}}, "kinds": []}"#,
        "definition `A`: property `x`: unknown type `date`",
    ),
    (
        r#"{"format": 1, "definitions": {"A": {"type": "object", "properties": {}, "required": ["x"]}}, "kinds": []}"#,
        "definition `A`: required property `x` is not defined",
    ),
    (
        r#"{"format": 1, "definitions": {"A": {"type": "object", "properties": {}, "additionalProperties": {}}}, "kinds": []}"#,
        "definition `A`: an object with both `properties` and `additionalProperties`",
    ),
    (
        r#"{"format": 1, "definitions": {}, "kinds": [{"group": "", "version": "v1", "kind": "Pod", "definition": "Pod"}]}"#,
        "kind `` `v1` `Pod`: refers to the unknown definition `Pod`",
    ),
    (
        r#"{"format": 1, "definitions": {"A": {}}, "kinds": [{"group": "", "version": "v1", "kind": "A", "definition": "A"}, {"group": "", "version": "v1", "kind": "A", "definition": "A"}]}"#,
        "kind `` `v1` `A` is listed twice",
    ),
    (
        r#"{"format": 1, "definitions": {"A": {"oneOf": [{"type": "date"}]}}, "kinds": []}"#,
        "definition `A`: unknown type `date`",
    ),
    (
        r#"{"format": 1, "definitions": {"A": {"type": "object", "additionalProperties": {"type": "date"}}}, "kinds": []}"#,
        "definition `A`: unknown type `date`",
    ),
    (
        r#"{"format": 1, "definitions": {"A": {"type": "array", "items": {"$ref": "B"}}}, "kinds": []}"#,
        "definition `A`: refers to the unknown definition `B`",
    ),
    (
        r#"{"format": 1, "definitions": {"A": {"type": "object", "required": ["x"]}}, "kinds": []}"#,
        "definition `A`: `required` without `properties`",
    ),
    (
        r#"{"format": 1, "definitions": {"A": {"$ref": "B"}, "B": {"$ref": "A"}}, "kinds": []}"#,
        "definition `A` refers to itself",
    ),
    (
        r#"{"format": 1, "definitions": {"A": {"$ref": "A"}}, "kinds": []}"#,
        "definition `A` refers to itself",
    ),
];

#[test]
fn rejects_invalid_files() {
    for (json, expected) in INVALID {
        let error = Schema::from_json(json).unwrap_err();
        assert_eq!(error.to_string(), *expected, "{json}");
    }
    for json in [
        "{",
        r#"{"format": 1, "definitions": {"A": {"requried": []}}, "kinds": []}"#,
        r#"{"format": 1, "definitions": {"A": {"allOf": [{"$ref": "A"}]}}, "kinds": []}"#,
        r#"{"format": 1, "definitions": {}, "kinds": [], "extra": true}"#,
    ] {
        let error = Schema::from_json(json).unwrap_err();
        assert!(
            error.to_string().starts_with("invalid schema file: "),
            "{json}: {error}"
        );
    }
}

#[test]
fn checks_the_format_before_the_fields() {
    let error = Schema::from_json(r#"{"format": 2, "types": {}}"#).unwrap_err();
    assert_eq!(
        error.to_string(),
        "schema format 2 is not the supported format 1"
    );
}

#[test]
fn accepts_references_that_reach_a_shape() {
    let json = r#"{"format": 1, "definitions": {"A": {"$ref": "B"}, "B": {"$ref": "C"}, "C": {"type": "object", "properties": {"self": {"$ref": "A"}}}}, "kinds": []}"#;
    assert!(Schema::from_json(json).is_ok());
}

#[test]
fn rejects_unparsable_text() {
    let error = Schema::from_json("{").unwrap_err();
    assert!(
        error.to_string().starts_with("invalid schema file: "),
        "{error}"
    );
}
