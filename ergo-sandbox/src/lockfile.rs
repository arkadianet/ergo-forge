//! Reproducible compilation records, not deployment provenance or validation.
use crate::{
    identity::LIMITATION,
    map::source::{ChainBox, ChainSource},
    SandboxError, TypedValue,
};
use ergo_ser::address::NetworkPrefix;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Lockfile {
    pub schema_version: u32,
    pub source_sha256: String,
    pub params: BTreeMap<String, TypedValue>,
    pub compiler_revision: String,
    pub node_revision: String,
    pub workbench_version: String,
    pub tree_hex: String,
    pub address: String,
    pub tree_version: u8,
    pub lint_sweep_digest: String,
    pub limitation: String,
}

/// SHA-256 of compact UTF-8 JSON arrays `[lint_id,node_id,message]`, sorted
/// lexicographically by lint id, numerically by node id, then by message.
/// Duplicates are retained. Lift uses mainnet rendering on every network, so
/// a network choice cannot change the lint projection for identical bytes.
/// An empty list hashes `[]`. Findings remain static review priorities.
pub fn lint_sweep_digest(tree: &ergo_ser::ergo_tree::ErgoTree) -> String {
    let lifted = crate::lift_tree(tree, false);
    let audit = crate::audit::audit(&lifted);
    let mut rows: Vec<_> = audit
        .findings
        .iter()
        .map(|f| (f.lint, f.node_id, f.message.as_str()))
        .collect();
    rows.sort();
    sha256(&serde_json::to_vec(&rows).expect("finding tuples serialize"))
}
fn sha256(bytes: &[u8]) -> String {
    hex::encode(Sha256::digest(bytes))
}

/// How the requested tree version reaches the header of the locked tree.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum HeaderVersion {
    /// As every compile: the compiler's v0 header, unless the script uses a v6
    /// method, which gets the requested version (`compile::stamp_version`).
    #[default]
    AsNeeded,
    /// Exactly the requested version, with the size flag from v1 on, as a
    /// wallet stamps it and as an EIP-5 template's header already is:
    /// `ergo-es lock --tree-version 3` locks a v3 tree for any script. Only the
    /// header changes; the proposition, so the P2SH address, does not.
    Requested,
}

/// Run on the engine large stack. Source bytes are hashed without trimming or
/// newline normalization. Parameters preserve the supplied typed JSON values.
pub fn create(
    source: &str,
    params: &BTreeMap<String, TypedValue>,
    tree_version: u8,
    network: NetworkPrefix,
) -> Result<Lockfile, SandboxError> {
    create_with_header(
        source,
        params,
        tree_version,
        network,
        HeaderVersion::AsNeeded,
    )
}

/// [`create`] with the header version chosen by `header`.
pub fn create_with_header(
    source: &str,
    params: &BTreeMap<String, TypedValue>,
    tree_version: u8,
    network: NetworkPrefix,
    header: HeaderVersion,
) -> Result<Lockfile, SandboxError> {
    let mut compiled = crate::compile::compile_with_params(source, params, tree_version, network)?;
    if header == HeaderVersion::Requested
        && tree_version > 0
        && compiled.ergo_tree.version != tree_version
    {
        compiled.ergo_tree.version = tree_version;
        compiled.ergo_tree.has_size = true;
        let mut w = ergo_primitives::writer::VlqWriter::new();
        ergo_ser::ergo_tree::write_ergo_tree(&mut w, &compiled.ergo_tree)
            .map_err(|e| SandboxError::Tree(format!("v{tree_version} header: {e:?}")))?;
        compiled.tree_bytes = w.result();
        compiled.p2s_address = ergo_ser::address::encode_p2s(network, &compiled.tree_bytes);
    }
    Ok(Lockfile {
        schema_version: 1,
        source_sha256: sha256(source.as_bytes()),
        params: params.clone(),
        compiler_revision: crate::evidence::case::engine_revision().into(),
        node_revision: crate::evidence::case::engine_revision().into(),
        workbench_version: env!("CARGO_PKG_VERSION").into(),
        lint_sweep_digest: lint_sweep_digest(&compiled.ergo_tree),
        tree_hex: hex::encode(compiled.tree_bytes),
        address: compiled.p2s_address,
        tree_version: compiled.ergo_tree.version,
        limitation: LIMITATION.into(),
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DriftKind {
    SchemaVersion,
    Source,
    Params,
    CompilerRevision,
    NodeRevision,
    WorkbenchVersion,
    TreeBytes,
    Address,
    TreeVersion,
    LintDigest,
    Limitation,
}
#[derive(Debug, Serialize)]
pub struct Drift {
    pub kind: DriftKind,
    pub field: &'static str,
    pub locked: Value,
    pub current: Value,
}
#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum LockStatus {
    Current,
    Drift,
}
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VerifyLockReport {
    #[serde(flatten)]
    pub claim: crate::claim::ClaimMetadata,
    pub status: LockStatus,
    pub drifts: Vec<Drift>,
    pub live: LiveComparison,
    pub limitation: &'static str,
}

/// Compare each recorded field independently against a fresh build. Options
/// and params come from the caller, never from a potentially drifted field.
/// Without a separate source observation, the live result stays unverified.
pub fn verify_lock(
    locked: &Lockfile,
    source: &str,
    params: &BTreeMap<String, TypedValue>,
    tree_version: u8,
    network: NetworkPrefix,
) -> Result<VerifyLockReport, SandboxError> {
    verify_lock_with_header(
        locked,
        source,
        params,
        tree_version,
        network,
        HeaderVersion::AsNeeded,
    )
}

/// [`verify_lock`] with the header version chosen by `header`, which, like the
/// other options, comes from the caller and never from the lock.
pub fn verify_lock_with_header(
    locked: &Lockfile,
    source: &str,
    params: &BTreeMap<String, TypedValue>,
    tree_version: u8,
    network: NetworkPrefix,
    header: HeaderVersion,
) -> Result<VerifyLockReport, SandboxError> {
    let current = create_with_header(source, params, tree_version, network, header)?;
    let a = serde_json::to_value(locked).expect("lock serializes");
    let b = serde_json::to_value(current).expect("lock serializes");
    let fields = [
        ("schemaVersion", DriftKind::SchemaVersion),
        ("sourceSha256", DriftKind::Source),
        ("params", DriftKind::Params),
        ("compilerRevision", DriftKind::CompilerRevision),
        ("nodeRevision", DriftKind::NodeRevision),
        ("workbenchVersion", DriftKind::WorkbenchVersion),
        ("treeHex", DriftKind::TreeBytes),
        ("address", DriftKind::Address),
        ("treeVersion", DriftKind::TreeVersion),
        ("lintSweepDigest", DriftKind::LintDigest),
        ("limitation", DriftKind::Limitation),
    ];
    let drifts: Vec<_> = fields
        .into_iter()
        .filter(|(f, _)| a[f] != b[f])
        .map(|(field, kind)| Drift {
            kind,
            field,
            locked: a[field].clone(),
            current: b[field].clone(),
        })
        .collect();
    Ok(VerifyLockReport {
        claim: crate::claim::ClaimMetadata::STATIC,
        status: if drifts.is_empty() {
            LockStatus::Current
        } else {
            LockStatus::Drift
        },
        drifts,
        live: LiveComparison::unverified("No live box observation supplied."),
        limitation: LIMITATION,
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum LiveStatus {
    Unverified,
    BytesMatch,
    BytesDiffer,
}
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LiveComparison {
    #[serde(flatten)]
    pub claim: crate::claim::ClaimMetadata,
    pub status: LiveStatus,
    pub box_id: Option<String>,
    pub observed_tree_hex: Option<String>,
    pub reason: String,
    pub limitation: &'static str,
}
impl LiveComparison {
    pub fn unverified(reason: impl Into<String>) -> Self {
        Self {
            claim: crate::claim::ClaimMetadata::MAP,
            status: LiveStatus::Unverified,
            box_id: None,
            observed_tree_hex: None,
            reason: reason.into(),
            limitation: LIMITATION,
        }
    }
}

/// The SAME comparison is called for fixture and explorer ChainBox responses.
/// Byte equality is an observation about supplied data, not chain membership.
pub fn compare_live_box(locked: &Lockfile, box_data: &ChainBox) -> LiveComparison {
    let (Ok(a), Ok(b)) = (
        hex::decode(&locked.tree_hex),
        hex::decode(&box_data.ergo_tree),
    ) else {
        return LiveComparison::unverified("Invalid tree hex in lock or box response.");
    };
    if a.is_empty() || b.is_empty() {
        return LiveComparison::unverified("Empty tree in lock or box response.");
    }
    LiveComparison {
        claim: crate::claim::ClaimMetadata::MAP,
        status: if a == b { LiveStatus::BytesMatch } else { LiveStatus::BytesDiffer },
        box_id: Some(box_data.box_id.clone()), observed_tree_hex: Some(hex::encode(b)),
        reason: "Compared locked bytes with supplied box bytes; chain membership and currentness are not independently verified.".into(),
        limitation: LIMITATION,
    }
}

/// One bounded NFT-holder lookup. Missing explorer, gaps, ambiguity, or failed
/// transport never turn into a matching live result. No default public URL.
pub fn compare_live(
    locked: &Lockfile,
    nft: Option<&str>,
    source: Option<&dyn ChainSource>,
) -> LiveComparison {
    let Some(nft) = nft else {
        return LiveComparison::unverified("No --nft supplied.");
    };
    if nft.len() != 64 || hex::decode(nft).is_err() {
        return LiveComparison::unverified("NFT must be 32 bytes of hex.");
    }
    let Some(source) = source else {
        return LiveComparison::unverified("No explorer configured (explorer-dependency).");
    };
    let nft = nft.to_ascii_lowercase();
    let info = match source.token_info(&nft) {
        Ok(info) if info.id.eq_ignore_ascii_case(&nft) && info.is_singleton() => info,
        Ok(_) => return LiveComparison::unverified("Token is not a recorded singleton NFT."),
        Err(e) => return LiveComparison::unverified(e.to_string()),
    };
    let page = match source.boxes_by_token_id(&info.id, 0, 2) {
        Ok(p) => p,
        Err(e) => return LiveComparison::unverified(e.to_string()),
    };
    if page.items.len() != 1 || page.total.is_some_and(|n| n != 1) {
        return LiveComparison::unverified(
            "NFT holder missing, ambiguous, or response incomplete.",
        );
    }
    let b = &page.items[0];
    let tokens: Vec<_> = b
        .tokens
        .iter()
        .filter(|t| t.id.eq_ignore_ascii_case(&nft))
        .collect();
    if tokens.len() != 1
        || tokens[0].amount != 1
        || b.box_id.len() != 64
        || hex::decode(&b.box_id).is_err()
    {
        return LiveComparison::unverified(
            "Response does not identify one box holding one unit of the requested NFT.",
        );
    }
    compare_live_box(locked, b)
}
