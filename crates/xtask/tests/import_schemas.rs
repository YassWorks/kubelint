use std::collections::BTreeSet;

use kubelint_schema::Schema;
use xtask::schemas::{self, Source};

const FIXTURES: [(&str, &str); 3] = [
    (
        "api__v1_openapi.json",
        include_str!("../resources/test/openapi/api__v1_openapi.json"),
    ),
    (
        "apis__apps__v1_openapi.json",
        include_str!("../resources/test/openapi/apis__apps__v1_openapi.json"),
    ),
    (
        "version_openapi.json",
        include_str!("../resources/test/openapi/version_openapi.json"),
    ),
];

fn documents(files: &[(&str, &str)]) -> Vec<(String, String)> {
    files
        .iter()
        .map(|(name, text)| ((*name).to_owned(), (*text).to_owned()))
        .collect()
}

fn import_error(schema: &str) -> String {
    let document = format!(r#"{{"components": {{"schemas": {schema}}}}}"#);
    schemas::import(&documents(&[("test.json", &document)])).expect_err(schema)
}

#[test]
fn imports_the_fixture_documents() {
    let text = schemas::import(&documents(&FIXTURES)).expect("the fixtures import");
    insta::assert_snapshot!(text);
}

#[test]
fn reports_what_the_importer_cannot_read() {
    const KIND: &str =
        r#""x-kubernetes-group-version-kind": [{"group": "", "version": "v1", "kind": "A"}]"#;
    let cases = [
        (
            format!(r#"{{"A": {{"type": "object", "enum": ["a"], {KIND}}}}}"#),
            "unsupported keyword `enum`",
        ),
        (
            format!(
                r#"{{"A": {{"allOf": [{{"type": "string"}}, {{"type": "integer"}}], {KIND}}}}}"#
            ),
            "an `allOf` that is not a single schema",
        ),
        (
            format!(r#"{{"A": {{"allOf": [{{"type": "string"}}], "type": "object", {KIND}}}}}"#),
            "`allOf` next to `type`",
        ),
        (
            format!(r##"{{"A": {{"$ref": "#/definitions/B", {KIND}}}}}"##),
            "`$ref` outside `#/components/schemas/`",
        ),
        (
            format!(r##"{{"A": {{"$ref": "#/components/schemas/B", "type": "string", {KIND}}}}}"##),
            "`$ref` next to `type`",
        ),
        (
            r#"{"A": {"type": "object"}}"#.to_owned(),
            "no document defines a kind",
        ),
        (
            format!(r##"{{"A": {{"$ref": "#/components/schemas/B", {KIND}}}}}"##),
            "refers to the unknown definition `B`",
        ),
        (
            format!(r#"{{"A": {{"type": "object", {KIND}}}, "B": {{"type": "object", {KIND}}}}}"#),
            "kind `` `v1` `A` is defined by both `A` and `B`",
        ),
    ];
    for (schema, expected) in cases {
        let error = import_error(&schema);
        assert!(error.contains(expected), "{schema}: {error}");
    }
}

#[test]
fn a_definition_must_match_in_every_document() {
    let kind =
        r#""x-kubernetes-group-version-kind": [{"group": "", "version": "v1", "kind": "A"}]"#;
    let first =
        format!(r#"{{"components": {{"schemas": {{"A": {{"type": "string", {kind}}}}}}}}}"#);
    let second =
        format!(r#"{{"components": {{"schemas": {{"A": {{"type": "integer", {kind}}}}}}}}}"#);
    let error = schemas::import(&documents(&[("one.json", &first), ("two.json", &second)]))
        .expect_err("the definitions differ");
    assert_eq!(
        error,
        "definition `A` differs between one.json and two.json"
    );
}

#[test]
fn minor_versions_come_from_release_tags_only() {
    for (tag, expected) in [
        ("v1.37.2", Some("1.37")),
        ("v1.34.0", Some("1.34")),
        ("v10.100.1000", Some("10.100")),
        ("1.37.2", None),
        ("v1.37", None),
        ("v1.38.0-beta.0", None),
        ("v1.37.2.1", None),
        ("v1..2", None),
        ("v1.3a.2", None),
    ] {
        assert_eq!(schemas::minor_version(tag), expected, "{tag}");
    }
}

#[test]
fn sources_round_trip() {
    let mut sources = schemas::read_sources("").expect("an empty file has no sources");
    sources.insert(
        "1.37".to_owned(),
        Source {
            tag: "v1.37.2".to_owned(),
            commit: "c50e5395f1ae2d4a2c05f28fdcc38c0916cc1a32".to_owned(),
        },
    );
    let text = schemas::write_sources(&sources);
    insta::assert_snapshot!(text);
    assert_eq!(schemas::read_sources(&text), Ok(sources));
}

#[test]
fn committed_schemas_have_the_current_format() {
    for path in schemas::json_files(&xtask::repository_root().join(schemas::SCHEMAS_DIR))
        .expect("the schemas can be listed")
    {
        let text = std::fs::read_to_string(&path).expect("a schema file can be read");
        if let Err(error) = Schema::from_json(&text) {
            panic!(
                "{}: {error}. Run `cargo xtask import-schemas` again for this version.",
                path.display()
            );
        }
    }
}

#[test]
fn every_committed_schema_has_a_source() {
    let root = xtask::repository_root();
    let versions: BTreeSet<String> = schemas::json_files(&root.join(schemas::SCHEMAS_DIR))
        .expect("the schemas can be listed")
        .iter()
        .filter_map(|path| path.file_stem()?.to_str().map(str::to_owned))
        .collect();
    let text = std::fs::read_to_string(root.join(schemas::SCHEMAS_DIR).join("sources.toml"))
        .expect("sources.toml can be read");
    let sources: BTreeSet<String> = schemas::read_sources(&text)
        .expect("sources.toml is valid")
        .into_keys()
        .collect();
    assert_eq!(versions, sources);
}
