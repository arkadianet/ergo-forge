//! Read-only protocol traversal over the engine's public ChainSource API.
use std::{collections::BTreeMap, sync::Arc};

use axum::{extract::State, Json};
use ergo_sandbox::map::{self, source::*, Fixture, MapOptions, Seed, Target};
use serde::{Deserialize, Serialize};

use crate::{app::AppState, error::ApiError, extract::ApiJson};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MapRequest {
    pub input: String,
    pub kind: SeedKind,
    pub network: Option<String>,
    pub max_nodes: Option<usize>,
    pub max_depth: Option<u32>,
    /// An explicitly supplied recording never makes outbound calls.
    pub fixture: Option<serde_json::Value>,
}

#[derive(Clone, Copy, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum SeedKind {
    BoxId,
    Address,
    TokenId,
    TransactionId,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MapResponse {
    #[serde(flatten)]
    claim: ergo_sandbox::claim::ClaimMetadata,
    seed: SeedDto,
    network: String,
    source: SourceDto,
    caps: CapsDto,
    truncated: Option<TruncationDto>,
    nodes: Vec<NodeDto>,
    edges: Vec<EdgeDto>,
}
#[derive(Serialize)]
struct SeedDto {
    kind: SeedKind,
    input: String,
}
#[derive(Serialize)]
struct SourceDto {
    kind: String,
    height: u32,
    recorded: bool,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct CapsDto {
    max_nodes: usize,
    max_depth: u32,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct TruncationDto {
    nodes: usize,
    depth: usize,
    frontier: usize,
    per_token: BTreeMap<String, usize>,
    per_script_hash: BTreeMap<String, usize>,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct NodeDto {
    box_id: String,
    tree_hex: String,
    value: String,
    depth: u32,
    complete: bool,
    tree_error: Option<String>,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct EdgeDto {
    from: String,
    to: Option<String>,
    unresolved: Option<String>,
    binding: String,
    covers: Vec<String>,
    site: String,
    singleton: Option<bool>,
}

pub async fn map_route(
    State(state): State<Arc<AppState>>,
    ApiJson(req): ApiJson<MapRequest>,
) -> Result<Json<MapResponse>, ApiError> {
    let network = req.network.as_deref().unwrap_or("mainnet");
    super::inspect::parse_network(Some(network))?;
    let input = req.input.trim().to_string();
    if matches!(req.kind, SeedKind::Address) {
        // Validate before a value is used in an upstream path.
        if input.is_empty() || !input.chars().all(|c| c.is_ascii_alphanumeric()) {
            return Err(ApiError::InvalidInput("expected a contract address".into()));
        }
        ergo_ser::address::decode_address_to_tree_bytes(
            &input,
            super::inspect::parse_network(Some(network))?,
        )
        .map_err(|e| ApiError::InvalidInput(format!("expected a contract address: {e}")))?;
    } else if input.len() != 64 || !input.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err(ApiError::InvalidInput(
            "box, token and transaction IDs must be 64 hex characters".into(),
        ));
    }
    let input = if matches!(req.kind, SeedKind::Address) {
        input
    } else {
        input.to_ascii_lowercase()
    };
    let max_nodes = req.max_nodes.unwrap_or(24);
    let max_depth = req.max_depth.unwrap_or(2);
    if !(1..=64).contains(&max_nodes) || max_depth > 4 {
        return Err(ApiError::InvalidInput(
            "map caps: maxNodes must be 1–64 and maxDepth 0–4".into(),
        ));
    }
    let recorded = req.fixture.is_some();
    let fixture = req
        .fixture
        .map(|v| {
            let f = Fixture::from_json(&v.to_string()).map_err(ApiError::InvalidInput)?;
            if f.format_version != 1 {
                return Err(ApiError::InvalidInput(
                    "unsupported fixture formatVersion; expected 1".into(),
                ));
            }
            Ok(f)
        })
        .transpose()?;
    if !recorded {
        if state.cfg.explorer_url.is_none() {
            return Err(ApiError::NotConfigured(
                "Live mapping needs EXPLORER_URL. Open a recorded chain fixture to map offline."
                    .into(),
            ));
        }
        if network != state.cfg.explorer_network {
            return Err(ApiError::InvalidInput(format!(
                "this explorer serves {}; select that network",
                state.cfg.explorer_network
            )));
        }
    }
    let network = network.to_string();
    let base = state.cfg.explorer_url.clone();
    state
        .engine
        .run(move || {
            let source: Box<dyn ChainSource> = match fixture {
                Some(f) => Box::new(f),
                None => Box::new(map::explorer::ExplorerSource::new(
                    base.as_deref().expect("checked above"),
                )),
            };
            // The engine has no BoxId seed variant. Adapt just the initial
            // address frontier to one exact box; every other query is delegated.
            let source = BoxSeedSource {
                source,
                seed_box: matches!(req.kind, SeedKind::BoxId).then(|| input.clone()),
            };
            let seed = match req.kind {
                SeedKind::BoxId | SeedKind::Address => Seed::Address(input.clone()),
                SeedKind::TokenId => Seed::TokenId(input.clone()),
                SeedKind::TransactionId => Seed::TransactionId(input.clone()),
            };
            let opts = MapOptions {
                max_nodes,
                max_depth,
                testnet: network == "testnet",
                ..Default::default()
            };
            let m = map::map(&source, &seed, &opts).map_err(|e| match e {
                map::MapError::Seed(s) | map::MapError::Source(SourceError::NotFound(s)) => {
                    ApiError::NotFound(s)
                }
                e if recorded => ApiError::InvalidInput(e.to_string()),
                e => ApiError::Upstream(e.to_string()),
            })?;
            Ok(Json(MapResponse {
                claim: ergo_sandbox::claim::ClaimMetadata::MAP,
                seed: SeedDto {
                    kind: req.kind,
                    input,
                },
                network,
                source: SourceDto {
                    kind: m.source_kind,
                    height: m.height,
                    recorded,
                },
                caps: CapsDto {
                    max_nodes,
                    max_depth,
                },
                truncated: (!m.truncated.is_empty()).then_some(TruncationDto {
                    nodes: m.truncated.nodes,
                    depth: m.truncated.depth,
                    frontier: m.truncated.frontier,
                    per_token: m.truncated.per_token,
                    per_script_hash: m.truncated.per_script_hash,
                }),
                nodes: m
                    .nodes
                    .values()
                    .map(|n| NodeDto {
                        box_id: n.chain_box.box_id.clone(),
                        tree_hex: n.chain_box.ergo_tree.clone(),
                        value: n.chain_box.value.to_string(),
                        depth: n.depth,
                        complete: n.complete && n.tree_error.is_none(),
                        tree_error: n.tree_error.clone(),
                    })
                    .collect(),
                edges: m
                    .edges
                    .iter()
                    .map(|e| {
                        let (to, unresolved) = match &e.to {
                            Target::Node(id) => (Some(id.clone()), None),
                            Target::Unresolved(reason) => (None, Some(reason.clone())),
                        };
                        EdgeDto {
                            from: e.from.clone(),
                            to,
                            unresolved,
                            binding: e.binding.as_str().into(),
                            covers: e.covers.iter().map(|c| c.as_str().into()).collect(),
                            site: e.site.clone(),
                            singleton: e.singleton,
                        }
                    })
                    .collect(),
            }))
        })
        .await
        .ok_or(ApiError::Internal)?
}

struct BoxSeedSource {
    source: Box<dyn ChainSource>,
    seed_box: Option<String>,
}
impl ChainSource for BoxSeedSource {
    fn kind(&self) -> &str {
        self.source.kind()
    }
    fn url(&self) -> Option<&str> {
        self.source.url()
    }
    fn height(&self) -> Result<u32, SourceError> {
        self.source.height()
    }
    fn box_by_id(&self, id: &str) -> Result<ChainBox, SourceError> {
        self.source.box_by_id(id)
    }
    fn boxes_by_address(
        &self,
        address: &str,
        offset: usize,
        limit: usize,
    ) -> Result<Page, SourceError> {
        if self.seed_box.as_deref() == Some(address) {
            return Ok(Page {
                items: if offset == 0 && limit > 0 {
                    vec![self.source.box_by_id(address)?]
                } else {
                    vec![]
                },
                total: Some(1),
            });
        }
        self.source.boxes_by_address(address, offset, limit)
    }
    fn boxes_by_token_id(
        &self,
        id: &str,
        offset: usize,
        limit: usize,
    ) -> Result<Page, SourceError> {
        self.source.boxes_by_token_id(id, offset, limit)
    }
    fn token_info(&self, id: &str) -> Result<TokenInfo, SourceError> {
        self.source.token_info(id)
    }
    fn boxes_by_script_hash(
        &self,
        hash: &str,
        offset: usize,
        limit: usize,
    ) -> Result<Page, SourceError> {
        self.source.boxes_by_script_hash(hash, offset, limit)
    }
    fn transaction(&self, id: &str) -> Result<TxBoxes, SourceError> {
        self.source.transaction(id)
    }
}
