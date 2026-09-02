//! Thin wrapper over the ONE compile primitive, `ergo_compiler::compile`
//! (`ergoscript-tooling-api.md` §3.5). No reimplementation.
//!
//! Two entry points: [`compile_source`] with an empty environment, and
//! [`compile_with_params`] for the way real contracts are actually
//! compiled — with named compile-time constants. The corpus of deployed
//! contracts uses two placeholder mechanisms, both supported here:
//!
//! 1. **Environment constants.** `$name` and bare `Name` identifiers resolve
//!    through the compiler's `ScriptEnv` (the node's `keysToEnv` path).
//! 2. **Textual substitution inside string literals.** `fromBase64("$x")`
//!    needs the text inside the string. A `String`-typed parameter is
//!    substituted wherever `"$x"` appears in a string literal; a
//!    `Coll[Byte]` parameter is substituted as hex there.
//!
//! [`scan_params`] lists the `$names` a source needs, with the corpus's
//! `// $name: Type` comment hints, so a UI can build a form before compiling.
//!
//! Design record: `docs/superpowers/specs/2026-09-03-playground-design.md`.

use std::collections::BTreeMap;

use ergo_compiler::{CompileError, CompileResult, EnvValue, NetworkPrefix, ScriptEnv};
use ergo_ser::sigma_type::SigmaType;
use ergo_ser::sigma_value::{CollValue, SigmaBoolean, SigmaValue};
use serde::Serialize;

use crate::scenario::{parse_typed_value, TypedValue};
use crate::SandboxError;

/// The output of a successful source compilation.
#[derive(Debug, Clone)]
pub struct CompileOutput {
    /// Canonical ErgoTree wire bytes.
    pub tree_bytes: Vec<u8>,
    /// Parsed tree.
    pub ergo_tree: ergo_ser::ergo_tree::ErgoTree,
    /// Pay-to-script address.
    pub p2s_address: String,
    /// Pay-to-script-hash address.
    pub p2sh_address: String,
}

/// Compile ErgoScript source into tree bytes + addresses, empty environment.
///
/// `network` affects only the address encodings, not the tree bytes.
pub fn compile_source(
    source: &str,
    tree_version: u8,
    network: NetworkPrefix,
) -> Result<CompileOutput, CompileError> {
    compile_env(&ScriptEnv::new(), source, tree_version, network)
}

fn compile_env(
    env: &ScriptEnv,
    source: &str,
    tree_version: u8,
    network: NetworkPrefix,
) -> Result<CompileOutput, CompileError> {
    let CompileResult {
        tree_bytes,
        ergo_tree,
        p2s_address,
        p2sh_address,
    } = ergo_compiler::compile(env, source, tree_version, network)?;
    Ok(CompileOutput {
        tree_bytes,
        ergo_tree,
        p2s_address,
        p2sh_address,
    })
}

/// Why a parameterised compile failed.
#[derive(Debug, thiserror::Error)]
pub enum ParamError {
    /// `$names` the source uses that no parameter supplies.
    #[error("missing parameters: {}", .0.join(", "))]
    Missing(Vec<String>),
    /// A parameter's value could not be parsed for its type, or the type is
    /// not one the environment can carry.
    #[error("parameter `{name}`: {reason}")]
    Value { name: String, reason: String },
    /// The compiler rejected the (substituted) source.
    #[error(transparent)]
    Compile(#[from] CompileError),
}

/// Compile with named compile-time parameters (see the module doc).
pub fn compile_with_params(
    source: &str,
    params: &BTreeMap<String, TypedValue>,
    tree_version: u8,
    network: NetworkPrefix,
) -> Result<CompileOutput, ParamError> {
    let needed = scan_params(source);
    let missing: Vec<String> = needed
        .iter()
        .filter(|n| !params.contains_key(&n.name))
        .map(|n| n.name.clone())
        .collect();
    if !missing.is_empty() {
        return Err(ParamError::Missing(missing));
    }

    let dollar_names: Vec<&str> = needed.iter().map(|n| n.name.as_str()).collect();
    let mut env = ScriptEnv::new();
    let mut text_subs: Vec<(String, String)> = Vec::new();
    for (name, tv) in params {
        if tv.r#type == "String" {
            let s = tv.value.as_str().ok_or_else(|| ParamError::Value {
                name: name.clone(),
                reason: "String parameters take a JSON string".into(),
            })?;
            text_subs.push((name.clone(), s.to_string()));
            continue;
        }
        let (tpe, value) =
            parse_typed_value(&tv.r#type, &tv.value).map_err(|e| ParamError::Value {
                name: name.clone(),
                reason: e.to_string(),
            })?;
        if let Some(text) = text_form(&tpe, &value) {
            text_subs.push((name.clone(), text));
        }
        let ev = env_value(&tpe, value).ok_or_else(|| ParamError::Value {
            name: name.clone(),
            reason: format!("type `{}` cannot be an environment constant", tv.r#type),
        })?;
        // A `$name` param binds `$name` only: sources commonly write
        // `val name = fromBase64("$name")`, and the bare name is theirs.
        // A param the scan did not see is a bare environment name.
        if dollar_names.contains(&name.as_str()) {
            env.insert(format!("${name}"), ev);
        } else {
            env.insert(name.clone(), ev);
        }
    }

    let substituted = substitute_in_strings(source, &text_subs);
    match compile_env(&env, &substituted, tree_version, network) {
        Ok(out) => Ok(out),
        Err(e) => match missing_env_name(&e) {
            Some(name) => Err(ParamError::Missing(vec![name])),
            None => Err(ParamError::Compile(e)),
        },
    }
}

/// The typer reports an unbound identifier as
/// "Cannot assign type for variable 'X' because it is not found in env".
/// That is a missing bare parameter, not a type error.
fn missing_env_name(e: &CompileError) -> Option<String> {
    let msg = e.to_string();
    let rest = msg.split("Cannot assign type for variable '").nth(1)?;
    let name = rest.split('\'').next()?;
    msg.contains("not found in env")
        .then(|| name.trim_start_matches('$').to_string())
}

/// Text used when a parameter is referenced inside a string literal.
fn text_form(tpe: &SigmaType, v: &SigmaValue) -> Option<String> {
    match (tpe, v) {
        (SigmaType::SColl(inner), SigmaValue::Coll(CollValue::Bytes(b)))
            if **inner == SigmaType::SByte =>
        {
            Some(hex::encode(b))
        }
        _ => None,
    }
}

/// Map a parsed sigma value onto the compiler's environment value space.
fn env_value(tpe: &SigmaType, v: SigmaValue) -> Option<EnvValue> {
    Some(match (tpe, v) {
        (SigmaType::SBoolean, SigmaValue::Boolean(b)) => EnvValue::Bool(b),
        (SigmaType::SByte, SigmaValue::Byte(x)) => EnvValue::Byte(x),
        (SigmaType::SShort, SigmaValue::Short(x)) => EnvValue::Short(x),
        (SigmaType::SInt, SigmaValue::Int(x)) => EnvValue::Int(x),
        (SigmaType::SLong, SigmaValue::Long(x)) => EnvValue::Long(x),
        (SigmaType::SBigInt, SigmaValue::BigInt(x)) => EnvValue::BigInt(x.to_string()),
        (SigmaType::SGroupElement, SigmaValue::GroupElement(g)) => EnvValue::GroupElement(g),
        (SigmaType::SSigmaProp, SigmaValue::SigmaProp(sb)) => match sb {
            SigmaBoolean::ProveDlog(pk) => EnvValue::ProveDlog(*pk.as_bytes()),
            SigmaBoolean::TrivialProp(b) => EnvValue::Bool(b),
            _ => return None,
        },
        (SigmaType::SColl(inner), SigmaValue::Coll(CollValue::Bytes(b)))
            if **inner == SigmaType::SByte =>
        {
            EnvValue::ByteArray(b.into_iter().map(|x| x as i8).collect())
        }
        (SigmaType::SColl(inner), SigmaValue::Coll(CollValue::Values(items)))
            if **inner == SigmaType::SLong =>
        {
            let mut longs = Vec::with_capacity(items.len());
            for it in items {
                match it {
                    SigmaValue::Long(l) => longs.push(l),
                    _ => return None,
                }
            }
            EnvValue::LongArray(longs)
        }
        _ => return None,
    })
}

/// Replace `$name` occurrences that sit inside string literals.
fn substitute_in_strings(source: &str, subs: &[(String, String)]) -> String {
    if subs.is_empty() {
        return source.to_string();
    }
    let mut out = String::with_capacity(source.len());
    let mut rest = source;
    while let Some(open) = rest.find('"') {
        out.push_str(&rest[..=open]);
        rest = &rest[open + 1..];
        let close = rest.find('"').unwrap_or(rest.len());
        let mut lit = rest[..close].to_string();
        for (name, text) in subs {
            lit = lit.replace(&format!("${name}"), text);
            if lit == *name {
                lit = text.clone();
            }
        }
        out.push_str(&lit);
        if close < rest.len() {
            out.push('"');
            rest = &rest[close + 1..];
        } else {
            rest = "";
        }
    }
    out.push_str(rest);
    out
}

/// A `$name` a source needs.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ParamNeed {
    /// The name without the `$`.
    pub name: String,
    /// Type from a `// $name: Type` comment, when the source carries one.
    pub type_hint: Option<String>,
}

/// List the parameters a source uses (outside comments, first use order):
/// `$name` identifiers, with type hints from `// $name: Type` comments, and
/// all-caps tokens that make up an entire string literal (`"RWT_REPO_NFT"`,
/// the deploy-script substitution style), hinted as `String`.
pub fn scan_params(source: &str) -> Vec<ParamNeed> {
    let (code, hints) = strip_comments(source);
    let mut out: Vec<ParamNeed> = Vec::new();
    let mut push = |name: &str, hint: Option<String>| {
        if !out.iter().any(|p| p.name == name) {
            out.push(ParamNeed {
                name: name.to_string(),
                type_hint: hint,
            });
        }
    };
    let bytes = code.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'"' {
            // A whole-literal caps token is a param; `$name`s inside strings
            // are scanned like anywhere else (below), so fall through.
            let close = code[i + 1..]
                .find('"')
                .map(|c| i + 1 + c)
                .unwrap_or(code.len());
            let lit = &code[i + 1..close];
            if is_caps_token(lit) {
                push(lit, Some("String".into()));
            }
            i += 1;
            continue;
        }
        if bytes[i] == b'$' {
            let start = i + 1;
            let mut end = start;
            while end < bytes.len() && (bytes[end].is_ascii_alphanumeric() || bytes[end] == b'_') {
                end += 1;
            }
            if end > start && !bytes[start].is_ascii_digit() {
                let name = &code[start..end];
                push(name, hints.get(name).cloned());
            }
            i = end.max(i + 1);
        } else {
            i += 1;
        }
    }
    out
}

/// `RWT_REPO_NFT`-shaped: upper-case letters, digits and underscores, at
/// least one underscore, starting with a letter.
fn is_caps_token(s: &str) -> bool {
    s.len() >= 4
        && s.starts_with(|c: char| c.is_ascii_uppercase())
        && s.contains('_')
        && s.chars()
            .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '_')
}

/// Remove `//` and `/* */` comments, collecting `$name: Type` hints from them.
fn strip_comments(source: &str) -> (String, BTreeMap<String, String>) {
    let mut code = String::with_capacity(source.len());
    let mut hints = BTreeMap::new();
    let mut rest = source;
    loop {
        let line = rest.find("//");
        let block = rest.find("/*");
        match (line, block) {
            (None, None) => {
                code.push_str(rest);
                break;
            }
            (Some(l), b) if b.is_none() || b.is_some_and(|b| l < b) => {
                code.push_str(&rest[..l]);
                let end = rest[l..].find('\n').map(|e| l + e).unwrap_or(rest.len());
                collect_hint(&rest[l + 2..end], &mut hints);
                code.push('\n');
                rest = if end < rest.len() {
                    &rest[end + 1..]
                } else {
                    ""
                };
            }
            (_, Some(b)) => {
                code.push_str(&rest[..b]);
                let end = rest[b..]
                    .find("*/")
                    .map(|e| b + e + 2)
                    .unwrap_or(rest.len());
                collect_hint(&rest[b + 2..end.saturating_sub(2).max(b + 2)], &mut hints);
                code.push(' ');
                rest = &rest[end..];
            }
            (Some(_), None) => unreachable!("handled by the guard above"),
        }
    }
    (code, hints)
}

/// `$name: Type` at the start of a comment (after whitespace) is a hint.
fn collect_hint(comment: &str, hints: &mut BTreeMap<String, String>) {
    let c = comment.trim();
    let Some(rest) = c.strip_prefix('$') else {
        return;
    };
    let name_end = rest
        .find(|ch: char| !(ch.is_ascii_alphanumeric() || ch == '_'))
        .unwrap_or(rest.len());
    let name = &rest[..name_end];
    let Some(after) = rest[name_end..].trim_start().strip_prefix(':') else {
        return;
    };
    let tpe: String = after.split_whitespace().next().unwrap_or("").to_string();
    if !name.is_empty() && !tpe.is_empty() {
        hints.entry(name.to_string()).or_insert(tpe);
    }
}

impl From<ParamError> for SandboxError {
    fn from(e: ParamError) -> Self {
        match e {
            ParamError::Compile(c) => SandboxError::Compile(c),
            other => SandboxError::Scenario(other.to_string()),
        }
    }
}
