//! The map's canonical JSON — the artifact everything downstream consumes.
//!
//! **Byte-identical for a given chain state and caps.** Two rules make that
//! true rather than aspirational:
//!
//! - No run metadata inside the canonical artifact. `chainSource` carries only
//!   what the map depends on (`kind`, `url`, `height`); the wall-clock
//!   `fetchedAt` and `durationMs` live in a separate `run` block that
//!   [`canonical`] omits entirely.
//! - Every array has a canonical sort key: `nodes` by `boxId`; `edges` by
//!   `(from, to, binding, site)`; `findings` by `(severity, lint, from, to)`;
//!   a node's `tokens` by token id.
//!
//! Rendering — a CLI table, later a graph in the Read pane — is a view over
//! this. The JSON is the artifact.

use serde_json::{json, Map, Value};

use super::{Edge, MapNode, ProtocolMap, SetFinding, Target, TokenClass};

/// Schema version of the map document.
pub const FORMAT_VERSION: u32 = 1;

/// The canonical document: no `run` block, every array sorted.
///
/// This is what a determinism check compares.
#[must_use]
pub fn canonical(m: &ProtocolMap) -> Value {
    let mut root = Map::new();
    root.extend(
        serde_json::to_value(crate::claim::ClaimMetadata::STATIC)
            .expect("static labels serialize")
            .as_object()
            .expect("labels are an object")
            .clone(),
    );
    root.insert("formatVersion".into(), json!(FORMAT_VERSION));
    root.insert(
        "seed".into(),
        json!({ "kind": m.seed.kind(), "value": m.seed.value() }),
    );
    let mut cs = Map::new();
    cs.insert("kind".into(), json!(m.source_kind));
    if let Some(u) = &m.source_url {
        cs.insert("url".into(), json!(u));
    }
    cs.insert("height".into(), json!(m.height));
    root.insert("chainSource".into(), Value::Object(cs));
    root.insert(
        "caps".into(),
        json!({
            "maxNodes": m.options.max_nodes,
            "maxDepth": m.options.max_depth,
            "maxBoxesPerToken": m.options.max_boxes_per_token,
            "maxBoxesPerScriptHash": m.options.max_boxes_per_script_hash,
            "maxFrontier": m.options.max_frontier,
            "pageSize": m.options.page_size,
            "maxFetchPerQuery": m.options.max_fetch_per_query,
        }),
    );
    root.insert(
        "nodes".into(),
        Value::Array(m.nodes.values().map(node_json).collect()),
    );
    root.insert(
        "edges".into(),
        Value::Array(m.edges.iter().map(|e| edge_json(m, e)).collect()),
    );
    root.insert(
        "findings".into(),
        Value::Array(m.findings.iter().map(|f| finding_json(m, f)).collect()),
    );
    root.insert(
        "tokens".into(),
        Value::Array(
            m.tokens
                .iter()
                .map(|(id, class)| {
                    let (kind, emission) = match class {
                        TokenClass::Singleton => ("singleton", Some(1u64)),
                        TokenClass::Fungible(n) => ("fungible", Some(*n)),
                        TokenClass::NotAToken => ("notAToken", None),
                    };
                    json!({
                        "id": id,
                        "class": kind,
                        "emissionAmount": emission,
                        "protocolNft": m.protocol_nfts.contains(id),
                    })
                })
                .collect(),
        ),
    );
    root.insert("truncated".into(), truncated_json(m));
    Value::Object(root)
}

/// The full document: [`canonical`] plus the explicitly non-canonical `run`
/// block. This is what the CLI prints and what an archive stores.
#[must_use]
pub fn document(m: &ProtocolMap) -> Value {
    let mut v = canonical(m);
    if let Value::Object(o) = &mut v {
        o.insert(
            "run".into(),
            json!({ "fetchedAt": m.run.fetched_at, "durationMs": m.run.duration_ms }),
        );
    }
    v
}

fn node_json(n: &MapNode) -> Value {
    let mut tokens: Vec<Value> = n
        .chain_box
        .tokens
        .iter()
        .enumerate()
        .map(|(i, t)| json!({ "id": t.id, "amount": t.amount, "index": i }))
        .collect();
    tokens.sort_by(|a, b| a["id"].as_str().cmp(&b["id"].as_str()));
    let mut o = Map::new();
    o.insert("boxId".into(), json!(n.chain_box.box_id));
    o.insert("nft".into(), json!(n.nft));
    o.insert("treeHash".into(), json!(n.tree_hash));
    o.insert("value".into(), json!(n.chain_box.value));
    o.insert("tokens".into(), Value::Array(tokens));
    o.insert("role".into(), json!(n.role.as_str()));
    o.insert("depth".into(), json!(n.depth));
    // `complete` is the lift's own verdict: false means part of this contract
    // was not analysed, so an absence of edges from it proves nothing.
    o.insert(
        "complete".into(),
        json!(n.complete && n.tree_error.is_none()),
    );
    if let Some(e) = &n.tree_error {
        o.insert("treeError".into(), json!(e));
    }
    Value::Object(o)
}

fn node_ref(m: &ProtocolMap, box_id: &str) -> Value {
    match m.nodes.get(box_id) {
        Some(n) => json!({ "boxId": n.chain_box.box_id, "nft": n.nft, "role": n.role.as_str() }),
        None => json!({ "boxId": box_id }),
    }
}

fn target_json(m: &ProtocolMap, t: &Target) -> Value {
    match t {
        Target::Node(id) => node_ref(m, id),
        Target::Unresolved(h) => json!({ "unresolved": h }),
    }
}

fn edge_json(m: &ProtocolMap, e: &Edge) -> Value {
    let mut o = Map::new();
    o.insert("from".into(), node_ref(m, &e.from));
    o.insert("to".into(), target_json(m, &e.to));
    o.insert("binding".into(), json!(e.binding.as_str()));
    if let Some(s) = e.singleton {
        o.insert(
            "tokenClass".into(),
            json!(if s { "singleton" } else { "fungible" }),
        );
    }
    o.insert(
        "covers".into(),
        Value::Array(e.covers.iter().map(|c| json!(c.as_str())).collect()),
    );
    o.insert("valueMath".into(), json!(e.value_math));
    o.insert("site".into(), json!(e.site));
    Value::Object(o)
}

fn finding_json(m: &ProtocolMap, f: &SetFinding) -> Value {
    json!({
        "lint": f.finding.lint,
        "triage": f.finding.triage,
        "severity": f.finding.severity.label(),
        "severityMeaning": "review-priority",
        "from": node_ref(m, &f.from),
        "to": target_json(m, &f.to),
        "message": f.finding.message,
        "snippet": f.finding.snippet,
    })
}

fn truncated_json(m: &ProtocolMap) -> Value {
    if m.truncated.is_empty() {
        return Value::Null;
    }
    json!({
        "nodes": m.truncated.nodes,
        "depth": m.truncated.depth,
        "frontier": m.truncated.frontier,
        "perToken": m.truncated.per_token,
        "perScriptHash": m.truncated.per_script_hash,
    })
}
