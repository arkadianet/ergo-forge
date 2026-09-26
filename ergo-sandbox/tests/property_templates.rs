//! Property templates: expansion produces declaration text that re-enters
//! `Declaration::parse`, with no route around the schema.

use ergo_sandbox::properties::schema::{Declaration, SchemaError};
use ergo_sandbox::properties::template::{
    Template, TemplateError, MAX_SLOTS, MAX_SLOT_VALUE_BYTES, MAX_TEMPLATE_BYTES, SCHEMA_VERSION,
};
use serde_json::{json, Value};
use std::collections::BTreeMap;

/// The declaration this template stands for, with its identity and provenance
/// left to slots.
const BASE: &str = r#"{"schemaVersion":"author-property:v1","propertyId":"{{property_id}}","revision":"1","contracts":{"reserve":{"script":"10010100d17300","compilerRevision":"pinned-author-compiler"}},"scope":{"kind":"transition"},"roles":{"after":{"cardinality":"all-matches","collection":"outputs","selector":{"hex":"10010100d17300","kind":"script"}},"before":{"cardinality":"exactly-one","collection":"inputs","selector":{"index":0,"kind":"position"}}},"registers":{"liability":{"index":4,"role":"before","unit":{"kind":"nano-erg"},"valueType":"long"}},"guard":{"op":"boolean","value":true},"assertion":{"left":{"left":{"op":"sum-erg","role":"after"},"op":"sub","right":{"op":"sum-erg","role":"before"}},"op":"ge","right":{"op":"integer","unit":{"kind":"nano-erg"},"value":"0"}},"authorizationPremises":[{"id":"funding","source":"{{source}}","statement":"Only supplied test funding credentials are available"}],"sources":["{{source}}"]}"#;

fn source() -> Value {
    json!({"origin":"hypothetical","reference":"local:author-review","sha256":"ab".repeat(32)})
}

fn slots() -> Value {
    json!({"property_id": "reserve-change", "source": source()})
}

fn document(slots: Value, declaration: &str) -> Value {
    json!({
        "schemaVersion": SCHEMA_VERSION,
        "name": "reserve-change",
        "slots": slots,
        "declaration": declaration,
    })
}

fn parsed(slots: Value, declaration: &str) -> Result<Template, TemplateError> {
    Template::parse(&document(slots, declaration).to_string())
}

fn declared(slots: Value, declaration: &str) -> Template {
    parsed(slots, declaration).expect("template is accepted")
}

fn rejected(slots: Value, declaration: &str) -> TemplateError {
    parsed(slots, declaration).expect_err("template was accepted")
}

/// The declaration the base stands for, resolved independently of expansion.
fn expected() -> Value {
    serde_json::from_str(
        &BASE
            .replace(r#""{{property_id}}""#, r#""reserve-change""#)
            .replace(r#""{{source}}""#, &source().to_string()),
    )
    .expect("expected declaration json")
}

/// A complete declaration base with one field left to a slot.
fn base_with(field: &str, placeholder: &str) -> String {
    let mut declaration = expected();
    declaration[field] = Value::String(placeholder.to_string());
    declaration.to_string()
}

fn nested_nots(depth: usize) -> Value {
    let mut expr = json!({"op": "boolean", "value": true});
    for _ in 0..depth {
        expr = json!({"op": "not", "arg": expr});
    }
    expr
}

fn schema_refusal(result: Result<Declaration, TemplateError>, expected: &str) {
    match result {
        Err(TemplateError::Declaration(SchemaError::Invalid(message))) => {
            assert!(message.contains(expected), "{message}");
        }
        other => panic!("expected {expected}, got {other:?}"),
    }
}

#[test]
fn a_template_expands_into_the_declaration_it_stands_for() {
    let t = declared(slots(), BASE);
    assert_eq!(t.name(), "reserve-change");
    assert_eq!(
        t.slots().keys().collect::<Vec<_>>(),
        vec!["property_id", "source"]
    );
    assert_eq!(
        t.placeholders().iter().collect::<Vec<_>>(),
        vec!["property_id", "source"]
    );
    let text = t.expand().expect("expand");
    assert!(!text.contains("{{"), "expansion left a placeholder: {text}");
    assert_eq!(
        serde_json::from_str::<Value>(&text).expect("expansion is json"),
        expected()
    );
    assert_eq!(
        t.declaration().expect("declaration").digest(),
        Declaration::parse(&expected().to_string())
            .expect("direct declaration")
            .digest()
    );
}

#[test]
fn expansion_is_byte_stable() {
    let t = declared(slots(), BASE);
    assert_eq!(t.expand().expect("expand"), t.expand().expect("expand"));
    assert_eq!(
        t.expand().expect("expand"),
        declared(slots(), BASE).expand().expect("expand")
    );
}

#[test]
fn a_slot_cannot_add_a_field_the_base_has_no_position_for() {
    let error = rejected(
        json!({"extra": "injected"}),
        r#"{"schemaVersion":"author-property:v1","propertyId":"p","{{extra}}":"x"}"#,
    );
    assert!(error.to_string().contains("object key"), "{error}");
}

#[test]
fn a_slot_cannot_select_another_declaration_version() {
    let base = BASE.replace("author-property:v1", "{{version}}");
    let t = declared(
        json!({"version": "author-property:v2", "property_id": "p", "source": source()}),
        &base,
    );
    assert_eq!(t.placeholders().len(), 3);
    schema_refusal(t.declaration(), "unsupported property version");
    let other = rejected(
        slots(),
        &BASE.replace("author-property:v1", "author-property:v2"),
    );
    assert!(matches!(other, TemplateError::Invalid(_)), "{other}");
}

#[test]
fn duplicate_keys_in_the_base_still_reach_the_schema() {
    // A repeated role key is invisible to the typed deserializer, which keeps
    // the last value; only the schema's own token scan refuses it.
    let base = BASE.replacen(
        r#""before":{"#,
        r#""before":{"cardinality":"all-matches","collection":"inputs","selector":{"index":1,"kind":"position"}},"before":{"#,
        1,
    );
    let t = declared(slots(), &base);
    schema_refusal(t.declaration(), "duplicate object key");
}

#[test]
fn a_slot_cannot_bypass_expression_typing() {
    let t = declared(
        json!({"guard": {"op": "integer", "unit": {"kind": "height"}, "value": "1"}}),
        &base_with("guard", "{{guard}}"),
    );
    schema_refusal(t.declaration(), "boolean");
}

#[test]
fn the_expression_node_ceiling_still_applies() {
    let base = base_with("assertion", "{{a}}");
    let at_ceiling = declared(json!({"a": nested_nots(30)}), &base);
    assert!(at_ceiling.declaration().is_ok());
    let over = declared(json!({"a": nested_nots(31)}), &base);
    match over.declaration() {
        Err(TemplateError::Declaration(SchemaError::Capped(message))) => {
            assert!(message.contains("32"), "{message}");
        }
        other => panic!("expected the node ceiling, got {other:?}"),
    }
}

#[test]
fn a_placeholder_must_be_whole_declared_and_named() {
    let partial = rejected(
        slots(),
        r#"{"schemaVersion":"author-property:v1","propertyId":"a-{{b"}"#,
    );
    assert!(
        partial.to_string().contains("whole string leaf"),
        "{partial}"
    );

    let inside = rejected(
        slots(),
        r#"{"schemaVersion":"author-property:v1","propertyId":"a {{b}} c"}"#,
    );
    assert!(inside.to_string().contains("whole string leaf"), "{inside}");

    let undeclared = rejected(json!({}), BASE);
    assert!(
        matches!(undeclared, TemplateError::Unresolved(_)),
        "{undeclared}"
    );

    let named = rejected(
        json!({"bad name": "x"}),
        r#"{"schemaVersion":"author-property:v1","propertyId":"{{bad name}}"}"#,
    );
    assert!(named.to_string().contains("alphanumeric"), "{named}");

    let braced = rejected(
        slots(),
        r#"{"schemaVersion":"author-property:v1","propertyId":"{{}}"}"#,
    );
    assert!(
        braced.to_string().contains("placeholder name length"),
        "{braced}"
    );

    let unterminated = rejected(
        slots(),
        r#"{"schemaVersion":"author-property:v1","propertyId":"{{a}"#,
    );
    assert!(
        unterminated.to_string().contains("unterminated string"),
        "{unterminated}"
    );
}

#[test]
fn the_template_document_itself_is_strict() {
    let mut wrong_version = document(slots(), BASE);
    wrong_version["schemaVersion"] = json!("property-template:v2");
    let err = Template::parse(&wrong_version.to_string()).expect_err("version accepted");
    assert!(
        err.to_string().contains("unsupported template version"),
        "{err}"
    );

    let mut extra = document(slots(), BASE);
    extra["extraField"] = json!(1);
    let err = Template::parse(&extra.to_string()).expect_err("unknown field accepted");
    assert!(err.to_string().contains("unknown field"), "{err}");

    let unnamed = json!({"schemaVersion": SCHEMA_VERSION, "slots": slots(), "declaration": BASE});
    let err = Template::parse(&unnamed.to_string()).expect_err("unnamed template accepted");
    assert!(err.to_string().contains("missing field `name`"), "{err}");

    let mut blank = document(slots(), BASE);
    blank["name"] = json!("  ");
    let err = Template::parse(&blank.to_string()).expect_err("blank name accepted");
    assert!(err.to_string().contains("empty template name"), "{err}");

    let not_object = rejected(slots(), "[1,2]");
    assert!(
        not_object.to_string().contains("JSON object"),
        "{not_object}"
    );

    let no_version = rejected(slots(), r#"{"propertyId":"p"}"#);
    assert!(
        no_version.to_string().contains("author-property:v1"),
        "{no_version}"
    );
}

#[test]
fn every_declared_bound_is_enforced_before_expansion() {
    let over_slots: BTreeMap<String, Value> = (0..=MAX_SLOTS)
        .map(|i| (format!("slot{i}"), json!(i)))
        .collect();
    let err = rejected(json!(over_slots), BASE);
    assert!(err.to_string().contains("declared slots"), "{err}");

    let big =
        json!({"property_id": "p", "source": source(), "big": "x".repeat(MAX_SLOT_VALUE_BYTES)});
    let err = rejected(big, BASE);
    assert!(err.to_string().contains("slot value bytes"), "{err}");

    let oversized = json!({
        "schemaVersion": SCHEMA_VERSION,
        "name": "n",
        "slots": {},
        "declaration": "x".repeat(MAX_TEMPLATE_BYTES),
    });
    assert!(oversized.to_string().len() > MAX_TEMPLATE_BYTES);
    let err = Template::parse(&oversized.to_string()).expect_err("oversized template accepted");
    assert!(err.to_string().contains("template bytes"), "{err}");

    let repeated = (0..5).map(|_| r#""{{big}}""#).collect::<Vec<_>>().join(",");
    let base = format!(r#"{{"schemaVersion":"author-property:v1","sources":[{repeated}]}}"#);
    let err = Template::parse(
        &document(json!({"big": "x".repeat(MAX_SLOT_VALUE_BYTES - 2)}), &base).to_string(),
    )
    .expect_err("oversized expansion accepted");
    assert!(
        err.to_string().contains("expanded declaration bytes"),
        "{err}"
    );
}
