//! Source ingestion for static tooling, with explicit synthetic bindings.
//!
//! A successful ingestion means the source compiled, its wire tree parsed, and
//! it lifted. It does NOT reproduce deployment bytes or preserve value-dependent
//! branches: the compiler can fold synthetic constants. Keep the report with the
//! artifact. No evaluation or analysis is performed here.
mod infer;

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use ergo_compiler::{CompileError, SType};
use ergo_ser::address::NetworkPrefix;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::compile::{compile_with_params, ParamError};
use crate::{lift_tree, Lifted, TypedValue};

/// A real value, or a known type for which ingestion supplies a placeholder.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(untagged, deny_unknown_fields)]
pub enum Override {
    Value(TypedValue),
    Type { r#type: String },
}

#[derive(Debug, Clone)]
pub struct IngestOptions {
    pub overrides: BTreeMap<String, Override>,
    pub tree_version: u8,
    pub network: NetworkPrefix,
    /// Disable inference for a reproducible empty-environment baseline.
    pub infer_constants: bool,
}

impl Default for IngestOptions {
    fn default() -> Self {
        Self {
            overrides: BTreeMap::new(),
            tree_version: 3,
            network: NetworkPrefix::Mainnet,
            infer_constants: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum BindingOrigin {
    Inferred,
    TypeOverride,
    ValueOverride,
}

#[derive(Debug, Clone, Serialize)]
pub struct Binding {
    pub parameter: String,
    pub bound: TypedValue,
    pub origin: BindingOrigin,
    /// One-based source lines containing free occurrences, excluding comments,
    /// strings, and locally bound occurrences.
    pub lines: Vec<usize>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Status {
    Compiled,
    NotCompiled,
}

#[derive(Debug, Clone, Serialize)]
pub struct ContractReport {
    pub path: PathBuf,
    pub status: Status,
    /// Full error, including compiler position and source line when available.
    pub reason: Option<String>,
    pub bindings: Vec<Binding>,
    pub notes: Vec<String>,
    pub requested_tree_version: u8,
    pub actual_tree_version: Option<u8>,
    pub raw_lift_nodes: Option<usize>,
    pub lift_truncated: Option<bool>,
}

/// Synthetic artifacts deliberately omit deployment addresses.
pub struct IngestedContract {
    pub tree_bytes: Vec<u8>,
    pub lifted: Lifted,
}

pub struct IngestResult {
    pub report: ContractReport,
    pub artifact: Option<IngestedContract>,
}

/// Compile, parse and lift source, binding only names the compiler identifies
/// as missing. Overrides always precede inference and are never retried with a
/// different type. Unknown or conflicting uses fail explicitly, without name
/// heuristics or a search for whichever value happens to compile.
pub fn ingest_source(source: &str, options: &IngestOptions) -> IngestResult {
    let mut report = ContractReport {
        path: PathBuf::new(),
        status: Status::NotCompiled,
        reason: None,
        bindings: vec![],
        notes: vec![],
        requested_tree_version: options.tree_version,
        actual_tree_version: None,
        raw_lift_nodes: None,
        lift_truncated: None,
    };
    if options.tree_version > 3 {
        return failed(report, "unsupported tree version: expected 0..=3".into());
    }
    let inference = if options.infer_constants {
        // A parser failure is reported by the compile path below, preserving
        // its error phase and position (templates use their existing path).
        let types = options
            .overrides
            .iter()
            .filter_map(|(n, o)| {
                let t = match o {
                    Override::Value(tv) => &tv.r#type,
                    Override::Type { r#type } => r#type,
                };
                ergo_compiler::parse_type(t, options.tree_version)
                    .ok()
                    .map(|t| (n.clone(), t))
            })
            .collect();
        infer::Inference::new(source, options.tree_version, &types).ok()
    } else {
        None
    };
    let mut params = BTreeMap::new();
    for (name, over) in &options.overrides {
        let (tv, origin) = match over {
            Override::Value(tv) => (tv.clone(), BindingOrigin::ValueOverride),
            Override::Type { r#type } => {
                let tv = match placeholder(name, r#type) {
                    Ok(tv) => tv,
                    Err(e) => return failed(report, format!("parameter `{name}`: {e}")),
                };
                (tv, BindingOrigin::TypeOverride)
            }
        };
        record(&mut report, source, inference.as_ref(), name, &tv, origin);
        params.insert(name.clone(), tv);
    }
    loop {
        match compile_with_params(source, &params, options.tree_version, options.network) {
            Ok(out) => {
                let tree = match crate::inspect::parse_tree(&out.tree_bytes) {
                    Ok(t) => t,
                    Err(e) => return failed(report, format!("compiled tree failed to parse: {e}")),
                };
                let lifted = lift_tree(&tree, options.network == NetworkPrefix::Testnet);
                report.status = Status::Compiled;
                report.actual_tree_version = Some(tree.version);
                report.raw_lift_nodes = Some(lifted.raw_placeholders);
                report.lift_truncated = Some(lifted.truncated);
                return IngestResult {
                    report,
                    artifact: Some(IngestedContract {
                        tree_bytes: out.tree_bytes,
                        lifted,
                    }),
                };
            }
            Err(ParamError::Missing(names)) if options.infer_constants => {
                for name in names {
                    if params.contains_key(&name) {
                        return failed(report,format!("parameter `{name}` remains unbound after supplying its override or inferred value"));
                    }
                    let inferred = inference
                        .as_ref()
                        .ok_or_else(|| "no AST inference available for this source".to_string())
                        .and_then(|i| i.inferred(&name));
                    let t = match inferred {
                        Ok(t) => t,
                        Err(e) => {
                            return failed(
                                report,
                                format!("parameter `{name}`: {e}; supply a type or value override"),
                            )
                        }
                    };
                    let type_name = ergo_compiler::typed_print::to_term_string(&t);
                    if !complete(&t) {
                        return failed(report,format!("parameter `{name}`: ambiguous usage, inferred only `{type_name}`; supply a type or value override"));
                    }
                    let tv = match placeholder(&name, &type_name) {
                        Ok(tv) => tv,
                        Err(e) => {
                            return failed(
                                report,
                                format!("parameter `{name}` inferred as `{type_name}`: {e}"),
                            )
                        }
                    };
                    record(
                        &mut report,
                        source,
                        inference.as_ref(),
                        &name,
                        &tv,
                        BindingOrigin::Inferred,
                    );
                    params.insert(name, tv);
                }
            }
            Err(e) => {
                let reason = diagnostic(source, &e);
                return failed(report, reason);
            }
        }
    }
}

fn complete(t: &SType) -> bool {
    match t {
        SType::NoType | SType::STypeVar(_) | SType::STypeApply { .. } => false,
        SType::SColl(t) | SType::SOption(t) => complete(t),
        SType::STuple(ts) => ts.iter().all(complete),
        SType::SFunc { .. } => false,
        _ => true,
    }
}

fn record(
    report: &mut ContractReport,
    source: &str,
    inference: Option<&infer::Inference>,
    name: &str,
    tv: &TypedValue,
    origin: BindingOrigin,
) {
    let mut lines: Vec<_> = inference
        .and_then(|i| {
            i.names
                .get(name)
                .or_else(|| i.names.get(&format!("${name}")))
        })
        .map(|(_, positions)| {
            positions
                .iter()
                .map(|p| source[..*p].bytes().filter(|b| *b == b'\n').count() + 1)
                .collect()
        })
        .unwrap_or_default();
    lines.sort_unstable();
    lines.dedup();
    if origin != BindingOrigin::ValueOverride && report.notes.is_empty() {
        report.notes.push("Static ingestion uses synthetic values and representative types; numeric contexts choose the widest observed width. Values, collection lengths and SigmaProp structure are assumptions. Compiler folding may change or remove value-dependent branches; this is not a deployment-equivalent tree.".into());
    }
    report.bindings.push(Binding {
        parameter: name.into(),
        bound: tv.clone(),
        origin,
        lines,
    });
}

fn failed(mut report: ContractReport, reason: String) -> IngestResult {
    report.reason = Some(reason);
    IngestResult {
        report,
        artifact: None,
    }
}

fn diagnostic(source: &str, e: &ParamError) -> String {
    if let ParamError::Compile(CompileError::Serializer { what }) = e {
        return format!("tree not serializable under a v0 header: {what}; the pinned compiler fixes the wire header at v0 even when tree_version=3 is requested. Header stamping happens only after serialization; no public compile option selects the serialization header.");
    }
    if let ParamError::Compile(
        c @ (CompileError::Parse(_) | CompileError::Bind(_) | CompileError::Type(_)),
    ) = e
    {
        let pos = (c.pos() as usize).min(source.len());
        let line = source.as_bytes()[..pos]
            .iter()
            .filter(|b| **b == b'\n')
            .count()
            + 1;
        return format!(
            "{e} at line {line}: {}",
            source.lines().nth(line - 1).unwrap_or("").trim()
        );
    }
    e.to_string()
}

fn placeholder(name: &str, t: &str) -> Result<TypedValue, String> {
    let digest = Sha256::digest(name.as_bytes());
    let value = match t {
        "Boolean" => serde_json::json!(false),
        "Byte" | "Short" | "Int" | "Long" => serde_json::json!(1 + digest[0] as i64 % 100),
        "BigInt" => serde_json::json!("1"),
        "Coll[Byte]" => serde_json::json!(hex::encode(digest)),
        "Coll[Long]" => serde_json::json!([1,2,3,4]),
        "SigmaProp" | "GroupElement" => {
            use k256::elliptic_curve::sec1::ToEncodedPoint;
            let key = k256::SecretKey::from_slice(&digest).map_err(|e| format!("cannot construct placeholder point: {e}"))?;
            serde_json::json!(hex::encode(key.public_key().to_encoded_point(true).as_bytes()))
        }
        _ => return Err(format!("no synthetic environment value supported for `{t}`; supply a supported real value or extend the compiler's ScriptEnv")),
    };
    Ok(TypedValue {
        r#type: t.into(),
        value,
    })
}

#[derive(Debug, Serialize)]
pub struct BatchReport {
    pub contracts: Vec<ContractReport>,
    pub compiled: usize,
    pub not_compiled: usize,
}

/// Recursively ingest `.ergo` and `.es` files in deterministic relative-path
/// order. Unreadable files are failure rows. Directory traversal errors abort
/// the batch rather than silently removing files from its denominator. Symlinks
/// are rejected to avoid cycles and accidentally omitting linked contracts.
pub fn ingest_directory(root: &Path, options: &IngestOptions) -> Result<BatchReport, String> {
    let mut paths = vec![];
    walk(root, &mut paths)?;
    paths.sort();
    let mut contracts = vec![];
    for path in paths {
        let mut report = match std::fs::read_to_string(&path) {
            Ok(source) => ingest_source(&source, options).report,
            Err(e) => ContractReport {
                path: PathBuf::new(),
                status: Status::NotCompiled,
                reason: Some(format!("read error: {e}")),
                bindings: vec![],
                notes: vec![],
                requested_tree_version: options.tree_version,
                actual_tree_version: None,
                raw_lift_nodes: None,
                lift_truncated: None,
            },
        };
        report.path = path.strip_prefix(root).unwrap_or(&path).to_path_buf();
        contracts.push(report);
    }
    let compiled = contracts
        .iter()
        .filter(|r| r.status == Status::Compiled)
        .count();
    let not_compiled = contracts.len() - compiled;
    Ok(BatchReport {
        contracts,
        compiled,
        not_compiled,
    })
}

fn walk(root: &Path, paths: &mut Vec<PathBuf>) -> Result<(), String> {
    for entry in std::fs::read_dir(root).map_err(|e| format!("{}: {e}", root.display()))? {
        let entry = entry.map_err(|e| format!("{}: {e}", root.display()))?;
        let p = entry.path();
        let t = entry
            .file_type()
            .map_err(|e| format!("{}: {e}", p.display()))?;
        if t.is_symlink() {
            return Err(format!(
                "{}: symlink encountered; use a directory containing regular source files",
                p.display()
            ));
        }
        if t.is_dir() {
            walk(&p, paths)?;
        } else if t.is_file() && p.extension().is_some_and(|e| e == "ergo" || e == "es") {
            paths.push(p);
        }
    }
    Ok(())
}
