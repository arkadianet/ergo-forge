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
use crate::evidence::{
    BindingSet, CasePremises, ConstantBinding, EvidenceCase, Origin, Premise, SourceIdentity,
};
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum BindingOrigin {
    Inferred,
    TypeOverride,
    ValueOverride,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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
    #[serde(flatten)]
    pub claim: crate::claim::ClaimMetadata,
    /// Provenance accompanies success and failure reports, including missing inputs.
    pub evidence_case: EvidenceCase,
    #[serde(serialize_with = "serialize_source_path")]
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

// Match the textual locator and sourcePath premise for non-UTF-8 filesystem paths.
fn serialize_source_path<S: serde::Serializer>(
    path: &Path,
    serializer: S,
) -> Result<S::Ok, S::Error> {
    serializer.serialize_str(&path.to_string_lossy())
}

/// Synthetic artifacts deliberately omit deployment addresses.
pub struct IngestedContract {
    case: EvidenceCase,
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
        evidence_case: source_case(Some(source), options),
        claim: crate::claim::ClaimMetadata::INGEST,
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
                bind_report(&mut report, Some(&out.tree_bytes));
                let case = report.evidence_case.clone();
                return IngestResult {
                    report,
                    artifact: Some(IngestedContract {
                        case,
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
    bind_report(&mut report, None);
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
    #[serde(flatten)]
    pub claim: crate::claim::ClaimMetadata,
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
                evidence_case: source_case(None, options),
                claim: crate::claim::ClaimMetadata::INGEST,
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
        let mut premises = report.evidence_case.premises().clone();
        if let Premise::Present { value, .. } = &mut premises.source {
            value.record.locator = report.path.to_string_lossy().into_owned();
        }
        premises.assumptions.insert(
            "sourcePath".into(),
            Premise::supplied(serde_json::json!(report.path.to_string_lossy())),
        );
        report.evidence_case = EvidenceCase::new(premises).expect("ingestion case remains valid");
        contracts.push(report);
    }
    let compiled = contracts
        .iter()
        .filter(|r| r.status == Status::Compiled)
        .count();
    let not_compiled = contracts.len() - compiled;
    Ok(BatchReport {
        claim: crate::claim::ClaimMetadata::INGEST,
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

impl IngestedContract {
    /// Legacy public bytes/IR may be read independently; evidence-bearing
    /// analysis rejects retargeted bytes and re-lifts the bound bytes itself.
    pub fn evidence_case(&self) -> Result<&EvidenceCase, String> {
        if self.case.target_bytes()? != self.tree_bytes {
            return Err("artifact bytes changed; resolve provenance for a new case".into());
        }
        Ok(&self.case)
    }
    pub fn analyze(
        &self,
    ) -> Result<crate::evidence::Analysis<crate::evidence::StaticAnalysis>, String> {
        self.evidence_case()?.analyze()
    }
}

fn source_case(source: Option<&str>, options: &IngestOptions) -> EvidenceCase {
    let mut premises = CasePremises::unspecified();
    if let Some(source) = source {
        premises.source = Premise::supplied(SourceIdentity::supplied(source));
    }
    premises.assumptions.insert("compilationOptions".into(), Premise::supplied(serde_json::json!({
        "treeVersion": options.tree_version,
        "network": if options.network == NetworkPrefix::Testnet { "testnet" } else { "mainnet" },
        "inferConstants": options.infer_constants,
        "overrides": options.overrides,
    })));
    EvidenceCase::new(premises).expect("source provenance is representable")
}

fn evidence_binding(b: &Binding) -> ConstantBinding {
    ConstantBinding {
        name: b.parameter.clone(),
        typed_value: serde_json::to_value(&b.bound).expect("binding serializes"),
        origin: if b.origin == BindingOrigin::ValueOverride {
            Origin::CallerSupplied
        } else {
            Origin::Hypothetical
        },
        mechanism: match b.origin {
            BindingOrigin::Inferred => "inferred",
            BindingOrigin::TypeOverride => "type-override",
            BindingOrigin::ValueOverride => "value-override",
        }
        .into(),
        source_lines: b.lines.clone(),
    }
}

fn bind_report(report: &mut ContractReport, target: Option<&[u8]>) {
    let mut premises = report.evidence_case.premises().clone();
    let bindings: Vec<_> = report.bindings.iter().map(evidence_binding).collect();
    let synthetic = bindings.iter().any(|b| b.origin == Origin::Hypothetical);
    premises.constants = Premise::supplied(BindingSet {
        bindings,
        complete: target.is_some(),
    });
    if let Some(target) = target {
        premises.target_bytes = Premise::Present {
            value: hex::encode(target),
            origin: if synthetic {
                Origin::Hypothetical
            } else {
                Origin::CallerSupplied
            },
        };
    }
    premises.assumptions.insert("compilationOutcome".into(), Premise::supplied(serde_json::json!({
        "status": report.status, "reason": report.reason, "actualTreeVersion": report.actual_tree_version,
        "rawLiftNodes": report.raw_lift_nodes, "liftTruncated": report.lift_truncated,
    })));
    report.evidence_case =
        EvidenceCase::new(premises).expect("captured bindings remain representable");
}

/// An archived compilation row is a measurement, not a retained compiler artifact.
/// Retain recorded binding values/origins; absent source text and target bytes
/// stay missing. Do not recompile private source to fill an old record.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RecordedInventoryRow {
    pub measurement: serde_json::Value,
    pub binding_origin_status: &'static str,
    pub case: EvidenceCase,
}

pub fn recorded_inventory(
    document: &serde_json::Value,
    locator: &str,
) -> Result<Vec<RecordedInventoryRow>, String> {
    use crate::evidence::SourceRecord;
    let files = document["files"]
        .as_array()
        .ok_or("inventory has no file identities")?;
    let rows = document["after"]["contracts"]
        .as_array()
        .ok_or("inventory has no compilation rows")?;
    let revision = document["corpus_revision"]
        .as_str()
        .ok_or("inventory has no corpus revision")?;
    let engine = document["compiler_revision"]
        .as_str()
        .ok_or("inventory has no compiler revision")?;
    let mut by_path = BTreeMap::new();
    for row in rows {
        let path = row["path"].as_str().ok_or("compilation row has no path")?;
        if by_path.insert(path, row).is_some() {
            return Err("duplicate compilation row".into());
        }
    }
    if files.len() != rows.len() {
        return Err("source and compilation inventories differ".into());
    }
    let mut output = Vec::new();
    for file in files {
        let path = file["path"].as_str().ok_or("source identity has no path")?;
        let row = by_path
            .remove(path)
            .ok_or("missing/duplicate source inventory member")?;
        let mut premises = CasePremises::unspecified();
        premises.engine_revision = engine.into();
        premises.source = Premise::Present {
            origin: Origin::SourceRecorded,
            value: SourceIdentity {
                record: SourceRecord {
                    locator: path.into(),
                    revision: Some(revision.into()),
                    sha256: file["sha256"].as_str().ok_or("source hash missing")?.into(),
                },
                text: Premise::missing("archived inventory records a hash, not source text"),
            },
        };
        let bindings: Vec<Binding> = serde_json::from_value(
            row.get("bindings")
                .ok_or("binding origins missing from inventory row")?
                .clone(),
        )
        .map_err(|e| format!("invalid recorded binding: {e}"))?;
        let complete = match row["status"].as_str() {
            Some("compiled") => true,
            Some("not_compiled") => false,
            _ => return Err("unknown compilation status".into()),
        };
        premises.constants = Premise::Present {
            origin: Origin::SourceRecorded,
            value: BindingSet {
                bindings: bindings.iter().map(evidence_binding).collect(),
                complete,
            },
        };
        premises.target_bytes = Premise::missing(
            "compiled bytes were not retained in this archived compilation summary",
        );
        premises.assumptions.insert(
            "compilationMeasurement".into(),
            Premise::Present {
                value: row.clone(),
                origin: Origin::SourceRecorded,
            },
        );
        premises.assumptions.insert("inventoryRecord".into(), Premise::Present {
            value: serde_json::json!({"locator":locator,"sha256":crate::evidence::case::json_digest(document)}), origin: Origin::SourceRecorded,
        });
        output.push(RecordedInventoryRow {
            measurement: row.clone(),
            binding_origin_status: "recorded",
            case: EvidenceCase::new(premises)?,
        });
    }
    Ok(output)
}
