//! Author templates that expand into an existing `author-property:v1` declaration.
//!
//! A template is a JSON document whose declaration text may carry `{{slot}}`
//! placeholders, each occupying a whole string value leaf. Expansion is a
//! value-for-value substitution, so the base fixes the document's field
//! positions and nesting; a placeholder in an object-key position is refused.
//! The expanded text then re-enters [`Declaration::parse`](super::schema::Declaration::parse),
//! where field rules, normalization, duplicate-key rejection and the fixed
//! ceilings apply exactly as they do to a hand-written declaration. Nothing
//! here builds a `Declaration` except that call.
use super::schema::{Declaration, SchemaError};
use serde::Deserialize;
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};

pub const SCHEMA_VERSION: &str = "property-template:v1";
pub const DECLARATION_VERSION: &str = "author-property:v1";
pub const MAX_TEMPLATE_BYTES: usize = 64 * 1024;
pub const MAX_SLOTS: usize = 16;
pub const MAX_SLOT_NAME: usize = 32;
pub const MAX_SLOT_VALUE_BYTES: usize = 4 * 1024;
pub const MAX_PLACEHOLDERS: usize = 32;
pub const MAX_EXPANSION_BYTES: usize = 16 * 1024;

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum TemplateError {
    /// The template is not usable as written.
    #[error("invalid template: {0}")]
    Invalid(String),
    /// A declared bound was exceeded; no expansion was attempted.
    #[error("unsupported/capped: {0}")]
    Capped(String),
    /// A placeholder in the declaration text has no declared slot.
    #[error("unresolved placeholder: {0}")]
    Unresolved(String),
    /// The expanded text is not a well-formed declaration. The schema's own
    /// error is carried unchanged.
    #[error("expanded declaration rejected: {0}")]
    Declaration(#[from] SchemaError),
}

type Result<T> = std::result::Result<T, TemplateError>;
fn invalid(s: impl Into<String>) -> TemplateError {
    TemplateError::Invalid(s.into())
}
fn capped(s: impl Into<String>) -> TemplateError {
    TemplateError::Capped(s.into())
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct TemplateInput {
    schema_version: String,
    name: String,
    #[serde(default)]
    slots: BTreeMap<String, Value>,
    declaration: String,
}

/// A bounded, checked template. The declaration stays unparsed text until
/// [`Template::declaration`] runs it through `Declaration::parse`.
#[derive(Debug, Clone)]
pub struct Template {
    input: TemplateInput,
    placeholders: BTreeSet<String>,
}

impl Template {
    pub fn parse(json: &str) -> Result<Self> {
        if json.len() > MAX_TEMPLATE_BYTES {
            return Err(capped("template bytes"));
        }
        let input: TemplateInput =
            serde_json::from_str(json).map_err(|e| invalid(e.to_string()))?;
        if input.schema_version != SCHEMA_VERSION {
            return Err(invalid("unsupported template version"));
        }
        if input.name.trim().is_empty() {
            return Err(invalid("empty template name"));
        }
        if input.slots.len() > MAX_SLOTS {
            return Err(capped("declared slots"));
        }
        for (name, value) in &input.slots {
            slot_name(name)?;
            let bytes = serde_json::to_vec(value)
                .map_err(|e| invalid(e.to_string()))?
                .len();
            if bytes > MAX_SLOT_VALUE_BYTES {
                return Err(capped("slot value bytes"));
            }
        }
        // The raw text is scanned before it is read as JSON, so a partial or
        // unterminated placeholder is reported as such.
        let mut placeholders = BTreeSet::new();
        let mut occurrences = 0usize;
        let mut projected = input.declaration.len();
        for (start, end) in literals(&input.declaration)? {
            let inner = inner_of(&input.declaration[start..end]);
            let Some(name) = slot_of(inner)? else {
                continue;
            };
            if input.declaration[end..]
                .trim_start_matches(char::is_whitespace)
                .starts_with(':')
            {
                return Err(invalid("a placeholder cannot occupy an object key"));
            }
            slot_name(name)?;
            let slot = input
                .slots
                .get(name)
                .ok_or_else(|| TemplateError::Unresolved(name.to_string()))?;
            let replacement = serde_json::to_string(slot).map_err(|e| invalid(e.to_string()))?;
            projected = projected
                .saturating_sub(end - start)
                .saturating_add(replacement.len());
            if projected > MAX_EXPANSION_BYTES {
                return Err(capped("expanded declaration bytes"));
            }
            placeholders.insert(name.to_string());
            occurrences += 1;
            if occurrences > MAX_PLACEHOLDERS {
                return Err(capped("placeholder occurrences"));
            }
        }
        // The base must already be the existing declaration vocabulary. This
        // `Value` read is only a well-formedness check: expansion works on the
        // original text, so duplicate keys survive to `Declaration::parse`
        // instead of being silently collapsed here.
        let base: Value =
            serde_json::from_str(&input.declaration).map_err(|e| invalid(e.to_string()))?;
        check_base(&base)?;
        Ok(Self {
            input,
            placeholders,
        })
    }

    pub fn name(&self) -> &str {
        &self.input.name
    }

    /// Declared slot values, by name. Unused declarations are permitted; the
    /// declaration text decides which slots an expansion consumes.
    pub fn slots(&self) -> &BTreeMap<String, Value> {
        &self.input.slots
    }

    /// Placeholder names the declaration text requires.
    pub fn placeholders(&self) -> &BTreeSet<String> {
        &self.placeholders
    }

    /// The declaration text this template produces.
    ///
    /// Text only, and not a declaration: callers wanting the checked form use
    /// [`Template::declaration`].
    pub fn expand(&self) -> Result<String> {
        let text = &self.input.declaration;
        let mut out = String::with_capacity(text.len());
        let mut copied = 0;
        for (start, end) in literals(text)? {
            let Some(name) = slot_of(inner_of(&text[start..end]))? else {
                continue;
            };
            let value = self
                .input
                .slots
                .get(name)
                .ok_or_else(|| TemplateError::Unresolved(name.to_string()))?;
            let value = serde_json::to_string(value).map_err(|e| invalid(e.to_string()))?;
            out.push_str(&text[copied..start]);
            out.push_str(&value);
            copied = end;
        }
        out.push_str(&text[copied..]);
        if out.len() > MAX_EXPANSION_BYTES {
            return Err(capped("expanded declaration bytes"));
        }
        Ok(out)
    }

    /// The checked declaration, obtained only by re-entering
    /// `Declaration::parse` on the expanded text.
    pub fn declaration(&self) -> Result<Declaration> {
        Ok(Declaration::parse(&self.expand()?)?)
    }
}

/// The base must declare the existing schema version. A version left as a
/// placeholder is settled after expansion, where the schema accepts only
/// `author-property:v1`.
fn check_base(base: &Value) -> Result<()> {
    let object = base
        .as_object()
        .ok_or_else(|| invalid("the declaration base must be a JSON object"))?;
    match object.get("schemaVersion") {
        Some(Value::String(version)) if version == DECLARATION_VERSION => Ok(()),
        Some(Value::String(version)) if slot_of(version)?.is_some() => Ok(()),
        _ => Err(invalid(
            "the declaration base must declare the author-property:v1 version",
        )),
    }
}

/// Byte spans of the string literals in `text`, each inclusive of its quotes.
fn literals(text: &str) -> Result<Vec<(usize, usize)>> {
    let bytes = text.as_bytes();
    let mut spans = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] != b'"' {
            i += 1;
            continue;
        }
        let start = i;
        i += 1;
        while i < bytes.len() && bytes[i] != b'"' {
            i += if bytes[i] == b'\\' { 2 } else { 1 };
        }
        if i >= bytes.len() {
            return Err(invalid("unterminated string in the declaration base"));
        }
        i += 1;
        spans.push((start, i));
    }
    Ok(spans)
}

fn inner_of(raw: &str) -> &str {
    &raw[1..raw.len() - 1]
}

/// The slot a string leaf names. A partial placeholder is refused rather than
/// copied through, so every substitution replaces a whole value.
fn slot_of(inner: &str) -> Result<Option<&str>> {
    if !inner.contains("{{") && !inner.contains("}}") {
        return Ok(None);
    }
    let name = inner
        .strip_prefix("{{")
        .and_then(|rest| rest.strip_suffix("}}"))
        .ok_or_else(|| invalid("a placeholder must occupy a whole string leaf"))?;
    Ok(Some(name))
}

fn slot_name(name: &str) -> Result<()> {
    if name.is_empty() || name.len() > MAX_SLOT_NAME {
        return Err(invalid("placeholder name length"));
    }
    if !name
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
    {
        return Err(invalid("placeholder name must be alphanumeric, `_` or `-`"));
    }
    Ok(())
}
