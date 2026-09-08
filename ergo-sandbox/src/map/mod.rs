//! The protocol map: **given one artifact of a deployed protocol, what is the
//! whole contract set, and how does each contract identify the others?**
//!
//! The insight the layer rests on is that the NFT constants compiled into an
//! ErgoTree *are* the protocol's dependency graph: a contract that must
//! recognise another box has that box's identity baked into its own bytes.
//! So the map is a breadth-first walk — decompile a tree, read its 32-byte
//! constants, recognise them as other contracts' NFTs, fetch the boxes
//! holding them, recurse.
//!
//! Nodes are boxes; **edges** are "contract A recognises box B", labelled with
//! *how*. The audit then runs on the edges, not only the nodes, because a
//! composition bug is in an edge: the 2026-09-08 USE LP drain happened
//! because the pool referenced the swap by NFT while the swap referenced the
//! pool by *position*. Neither tree is wrong alone.
//!
//! Design record: `docs/superpowers/specs/2026-09-08-protocol-map-design.md`.
//!
//! # Honest limits
//!
//! - **Dynamic references are invisible.** A collaborator found by scanning
//!   (`INPUTS.exists { … }`) or by a computed index has no constant to follow;
//!   the site is recorded as an unresolved reference, never omitted.
//! - **A map is of *now*** — one chain height, which is in the output.
//! - **Reachability is not completeness.** Absence from a map is not absence
//!   from the protocol.
//! - **Classification can be wrong.** A 32-byte constant that coincidentally
//!   matches a token id becomes an edge that is not one. The evidence is the
//!   archived chain source, so a human can overrule it.
//!
//! A map is a description, not a verdict.

#[cfg(feature = "explorer")]
pub mod explorer;
pub mod fixture;
pub mod json;
pub mod refs;
pub mod source;

use std::collections::{BTreeMap, BTreeSet, VecDeque};

use serde::Serialize;

use crate::audit::{Finding, Severity};
use refs::{tree_refs, Binding, BoxRef, Cover, TreeRefs};
use source::{ChainBox, ChainSource, SourceError};

pub use fixture::Fixture;
pub use refs::byte_constant;
pub use source::{ChainToken, Page, TokenInfo, TxBoxes};

/// Where a map starts. A seed is a starting point, never a trust anchor:
/// everything the chain source returns is re-parsed by the engine.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Seed {
    /// An NFT the protocol uses. Resolved to the boxes holding it.
    TokenId(String),
    /// An address; its unspent boxes, paginated and bounded by the frontier
    /// cap, form the initial frontier.
    Address(String),
    /// A transaction; its inputs and outputs form the initial frontier.
    TransactionId(String),
}

impl Seed {
    /// Guess a seed's kind from its spelling: 64 hex characters is an id
    /// (read as a token id — use [`Seed::TransactionId`] explicitly for a
    /// transaction), anything else is an address.
    #[must_use]
    pub fn guess(text: &str) -> Seed {
        let t = text.trim();
        if t.len() == 64 && t.chars().all(|c| c.is_ascii_hexdigit()) {
            Seed::TokenId(t.to_ascii_lowercase())
        } else {
            Seed::Address(t.to_string())
        }
    }

    fn kind(&self) -> &'static str {
        match self {
            Seed::TokenId(_) => "tokenId",
            Seed::Address(_) => "address",
            Seed::TransactionId(_) => "transactionId",
        }
    }

    fn value(&self) -> &str {
        match self {
            Seed::TokenId(v) | Seed::Address(v) | Seed::TransactionId(v) => v,
        }
    }
}

/// Traversal caps. Every one of them is recorded in the output and every
/// truncation is reported — a truncated map says so, in the same spirit as
/// the audit's `Completeness::Partial`.
#[derive(Debug, Clone)]
pub struct MapOptions {
    /// Most boxes admitted as nodes.
    pub max_nodes: usize,
    /// How far from the seed the walk goes. Depth 0 is the seed frontier.
    pub max_depth: u32,
    /// Most holders admitted per token id, after canonical ordering.
    pub max_boxes_per_token: usize,
    /// Most boxes admitted per script hash, after canonical ordering.
    pub max_boxes_per_script_hash: usize,
    /// Most boxes admitted in the initial frontier.
    pub max_frontier: usize,
    /// Page size asked of the chain source.
    pub page_size: usize,
    /// Most boxes fetched for one query before the map stops paging.
    ///
    /// A singleton NFT — the only kind that establishes identity — has one
    /// holder, so this never bites where it would matter. It bites on a
    /// fungible token with thousands of holders, where the selection is then
    /// over a bounded prefix of the chain's answer rather than all of it, and
    /// the per-token truncation reports the whole shortfall.
    pub max_fetch_per_query: usize,
    /// Encode `PK("…")` address constants for testnet when lifting.
    pub testnet: bool,
}

impl Default for MapOptions {
    fn default() -> Self {
        MapOptions {
            max_nodes: 64,
            max_depth: 4,
            max_boxes_per_token: 2,
            max_boxes_per_script_hash: 2,
            max_frontier: 16,
            page_size: 100,
            max_fetch_per_query: 32,
            testnet: false,
        }
    }
}

/// A box's inferred part in the protocol. Roles are a **proposal**, always
/// overridable, and `Unknown` is a real answer — never a silent default.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Role {
    /// Holds a protocol NFT and value that other contracts do arithmetic on.
    Protected,
    /// Holds a protocol NFT and only dust; its script is the authorisation.
    Companion,
    /// Read for its data rather than spent — an oracle or a tracker.
    External,
    /// The rules do not settle it. The caller resolves it; the drain hunt
    /// refuses to run on it.
    Unknown,
}

impl Role {
    /// Lowercase name used in the canonical JSON.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Role::Protected => "protected",
            Role::Companion => "companion",
            Role::External => "external",
            Role::Unknown => "unknown",
        }
    }
}

/// A companion holds its NFT and just enough ERG to exist. Above this the box
/// is carrying something, and `protected` becomes the question.
pub const COMPANION_MAX_VALUE: u64 = 1_000_000_000;

/// What the chain says about a 32-byte constant.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TokenClass {
    /// A token with this id exists, minted in quantity one: holding it
    /// identifies a single box. The only class that establishes identity.
    Singleton,
    /// A token with this id exists but is fungible. Referenced, and it may
    /// carry value, but any holder satisfies a check against it.
    Fungible(u64),
    /// The chain has no token with this id.
    NotAToken,
}

/// One box in the map.
#[derive(Debug, Clone)]
pub struct MapNode {
    /// The box, as the chain source returned it.
    pub chain_box: ChainBox,
    /// `blake2b256(propositionBytes)` — how a script-hash reference resolves.
    pub tree_hash: String,
    /// Distance from the seed frontier.
    pub depth: u32,
    /// What the box's tree says about the boxes it names.
    pub refs: TreeRefs,
    /// The whole tree lifted: no raw placeholders, no depth truncation. False
    /// means part of this contract was not analysed.
    pub complete: bool,
    /// Set when the tree could not be parsed or lifted at all.
    pub tree_error: Option<String>,
    /// The box's protocol NFT, if it has one. Filled after the walk, because
    /// "protocol NFT" is a property of the whole map.
    pub nft: Option<String>,
    /// Proposed role.
    pub role: Role,
}

/// Where an edge points.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum Target {
    /// A box in the map.
    Node(String),
    /// A reference that is real but resolves to nothing the map holds — a
    /// script hash with no matching tree and no source index, a token with no
    /// unspent holder, a dynamic site with no constant. Kept, because the
    /// reference is real and worth reporting, and **excluded from any
    /// completeness claim**.
    Unresolved(String),
}

impl Target {
    fn sort_key(&self) -> (u8, &str) {
        match self {
            Target::Node(id) => (0, id.as_str()),
            Target::Unresolved(h) => (1, h.as_str()),
        }
    }
}

/// "Contract A recognises box B", and how.
#[derive(Debug, Clone)]
pub struct Edge {
    /// Box id of the referring node.
    pub from: String,
    /// What it refers to.
    pub to: Target,
    /// How A establishes B's identity.
    pub binding: Binding,
    /// For an `nft` binding: whether the token is a singleton. Against a
    /// fungible token any holder qualifies, so the binding pins a token id
    /// and not a box.
    pub singleton: Option<bool>,
    /// Identity dimensions this reference pins. The rest stay open, and that
    /// is where this bug class lives.
    pub covers: BTreeSet<Cover>,
    /// A does arithmetic or comparison on B's `.value` or token amounts.
    pub value_math: bool,
    /// The slot in A's tree the reference is read at.
    pub site: String,
    /// Lift-local node id of the site, for anchoring a finding.
    pub node_id: u64,
}

impl Edge {
    /// Canonical sort key: `(from.boxId, to.boxId | targetHash, binding,
    /// site)`. The binding sorts by its **name**, not by the enum's
    /// strongest-first declaration order, so the order a JSON consumer sees
    /// is the one it can reproduce from the document alone.
    fn sort_key(&self) -> (&str, (u8, &str), &'static str, &str) {
        (
            &self.from,
            self.to.sort_key(),
            self.binding.as_str(),
            &self.site,
        )
    }
}

/// A set-level finding: an ordinary [`Finding`] attributed to an edge, so it
/// names the box actually at risk rather than only the tree that reads it.
#[derive(Debug, Clone)]
pub struct SetFinding {
    /// The lint result, in the audit layer's own shape.
    pub finding: Finding,
    /// Referring node's box id.
    pub from: String,
    /// What it refers to.
    pub to: Target,
}

/// What the caps cut off. Never silent.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Truncation {
    /// Boxes refused because `max_nodes` was reached.
    pub nodes: usize,
    /// Nodes left unexpanded because `max_depth` was reached.
    pub depth: usize,
    /// Seed boxes dropped from the initial frontier.
    pub frontier: usize,
    /// Holders omitted, per token id.
    pub per_token: BTreeMap<String, usize>,
    /// Boxes omitted, per script hash.
    pub per_script_hash: BTreeMap<String, usize>,
}

impl Truncation {
    /// Nothing was cut off.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        *self == Truncation::default()
    }
}

/// The map.
#[derive(Debug, Clone)]
pub struct ProtocolMap {
    /// What the map started from.
    pub seed: Seed,
    /// The chain source's kind, url and height — everything the map depends
    /// on, and nothing that varies between two runs on the same state.
    pub source_kind: String,
    /// Base URL, when the source has one.
    pub source_url: Option<String>,
    /// The chain height this map is of.
    pub height: u32,
    /// The caps this run used.
    pub options: MapOptions,
    /// Nodes, keyed by box id.
    pub nodes: BTreeMap<String, MapNode>,
    /// Edges, canonically sorted.
    pub edges: Vec<Edge>,
    /// Set-level findings, canonically sorted.
    pub findings: Vec<SetFinding>,
    /// Token evidence for every 32-byte constant the map classified.
    pub tokens: BTreeMap<String, TokenClass>,
    /// Token ids that are protocol NFTs: named by a mapped tree AND singleton.
    pub protocol_nfts: BTreeSet<String>,
    /// What the caps cut off.
    pub truncated: Truncation,
    /// Wall-clock, **non-canonical**. Excluded from comparison.
    pub run: RunMeta,
}

/// Run metadata. Deliberately outside the canonical artifact: two runs over
/// the same chain state differ here and only here.
#[derive(Debug, Clone, Default)]
pub struct RunMeta {
    /// Unix seconds the run started.
    pub fetched_at: u64,
    /// How long it took.
    pub duration_ms: u64,
}

/// Mapping failed before a map could form.
#[derive(Debug, thiserror::Error)]
pub enum MapError {
    /// The chain source could not answer a query the map depends on.
    #[error("{0}")]
    Source(#[from] SourceError),
    /// The seed did not resolve to anything to start from.
    #[error("seed: {0}")]
    Seed(String),
}

// ── the walk ─────────────────────────────────────────────────────────────────

/// Build a map, running the traversal on a stack large enough for deep trees.
///
/// The lift is a recursive descent and default thread stacks are marginal for
/// deeply nested contracts; this is the counterpart to `hunt`'s use of
/// [`crate::decompile::with_large_stack`].
pub fn map_owned<S: ChainSource + Send + 'static>(
    source: S,
    seed: Seed,
    opts: MapOptions,
) -> Result<ProtocolMap, MapError> {
    crate::decompile::with_large_stack(move || map(&source, &seed, &opts))
}

/// Build a map from `seed` against `source`.
pub fn map(
    source: &dyn ChainSource,
    seed: &Seed,
    opts: &MapOptions,
) -> Result<ProtocolMap, MapError> {
    let started = std::time::Instant::now();
    let fetched_at = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs());

    let mut w = Walk {
        source,
        opts,
        truncated: Truncation::default(),
        tokens: BTreeMap::new(),
        token_boxes: BTreeMap::new(),
        script_hash_boxes: BTreeMap::new(),
        script_hash_unsupported: false,
    };

    let height = source.height()?;
    let frontier = w.seed_frontier(seed)?;

    let mut nodes: BTreeMap<String, MapNode> = BTreeMap::new();
    // A box reached by two different references is one box: queued once, and
    // if the node cap refuses it, counted once.
    let mut queued: BTreeSet<String> = BTreeSet::new();
    let mut queue: VecDeque<(ChainBox, u32)> = VecDeque::new();
    for b in frontier {
        if queued.insert(b.box_id.clone()) {
            queue.push_back((b, 0u32));
        }
    }

    while let Some((b, depth)) = queue.pop_front() {
        if nodes.contains_key(&b.box_id) {
            continue;
        }
        if nodes.len() >= opts.max_nodes {
            w.truncated.nodes += 1;
            continue;
        }
        let node = build_node(&b, depth, opts.testnet);
        let constants = node.refs.constants.clone();
        nodes.insert(b.box_id.clone(), node);

        if depth >= opts.max_depth {
            w.truncated.depth += 1;
            continue;
        }
        for c in &constants {
            // A constant is followed as a token id when the chain says a token
            // with that id exists, and otherwise as a candidate script hash —
            // the two classifications the spec defines, in that order.
            let holders = if matches!(w.classify(c)?, TokenClass::NotAToken) {
                w.script_hash_boxes(c)?
            } else {
                w.token_boxes(c)?
            };
            for holder in holders {
                if !nodes.contains_key(&holder.box_id) && queued.insert(holder.box_id.clone()) {
                    queue.push_back((holder, depth + 1));
                }
            }
        }
    }

    // A node's own tokens need classifying too: the protocol-NFT rule reads
    // the emission amount of what a box *holds*, not only what trees name.
    let own: BTreeSet<String> = nodes
        .values()
        .flat_map(|n| n.chain_box.tokens.iter().map(|t| t.id.clone()))
        .collect();
    for id in &own {
        w.classify(id)?;
    }

    let Walk {
        truncated,
        tokens,
        token_boxes,
        ..
    } = w;

    // ── protocol NFTs: named by a mapped tree AND a singleton ───────────────
    let named: BTreeSet<String> = nodes
        .values()
        .flat_map(|n| n.refs.constants.iter().cloned())
        .collect();
    let protocol_nfts: BTreeSet<String> = named
        .iter()
        .filter(|c| tokens.get(*c) == Some(&TokenClass::Singleton))
        .cloned()
        .collect();

    for node in nodes.values_mut() {
        node.nft = protocol_nft_of(&node.chain_box, &protocol_nfts);
    }

    let mut edges = build_edges(&nodes, &token_boxes, &tokens);
    add_pairing_edges(&nodes, &mut edges);
    edges.sort_by(|a, b| a.sort_key().cmp(&b.sort_key()));
    edges.dedup_by(|a, b| a.sort_key() == b.sort_key());

    assign_roles(&mut nodes, &edges);

    let mut findings = set_findings(&nodes, &edges);
    findings.sort_by(|a, b| {
        (
            a.finding.severity,
            a.finding.lint,
            a.from.as_str(),
            a.to.sort_key(),
        )
            .cmp(&(
                b.finding.severity,
                b.finding.lint,
                b.from.as_str(),
                b.to.sort_key(),
            ))
    });

    Ok(ProtocolMap {
        seed: seed.clone(),
        source_kind: source.kind().to_string(),
        source_url: source.url().map(str::to_string),
        height,
        options: opts.clone(),
        nodes,
        edges,
        findings,
        tokens,
        protocol_nfts,
        truncated,
        run: RunMeta {
            fetched_at,
            duration_ms: started.elapsed().as_millis() as u64,
        },
    })
}

/// Traversal state: the source, the caps, and the caches that keep the map's
/// selection a property of the map rather than of the source's paging.
struct Walk<'a> {
    source: &'a dyn ChainSource,
    opts: &'a MapOptions,
    truncated: Truncation,
    tokens: BTreeMap<String, TokenClass>,
    token_boxes: BTreeMap<String, Vec<ChainBox>>,
    script_hash_boxes: BTreeMap<String, Vec<ChainBox>>,
    /// Set once the source has said it has no script-hash index, so the map
    /// asks once rather than once per constant.
    script_hash_unsupported: bool,
}

impl Walk<'_> {
    /// Fetch up to `max_fetch_per_query` boxes for a paginated query, then
    /// order them canonically by `(inclusionHeight, boxId)` and take the
    /// first `take`. The order is the map's, never the source's.
    fn select(
        &self,
        take: usize,
        mut fetch: impl FnMut(usize, usize) -> Result<source::Page, SourceError>,
    ) -> Result<(Vec<ChainBox>, usize), SourceError> {
        let mut all: Vec<ChainBox> = Vec::new();
        let mut total: Option<usize> = None;
        let mut offset = 0usize;
        while all.len() < self.opts.max_fetch_per_query {
            // `max(1)`: a zero page size would ask for nothing forever.
            let limit = self
                .opts
                .page_size
                .max(1)
                .min(self.opts.max_fetch_per_query - all.len());
            let page = fetch(offset, limit)?;
            let got = page.items.len();
            if total.is_none() {
                total = page.total;
            }
            all.extend(page.items);
            if got == 0 || got < limit {
                break;
            }
            offset += got;
        }
        all.sort_by(|a, b| a.order_key().cmp(&b.order_key()));
        all.dedup_by(|a, b| a.box_id == b.box_id);
        let known = total.unwrap_or(all.len()).max(all.len());
        let selected: Vec<ChainBox> = all.into_iter().take(take).collect();
        Ok((selected, known.saturating_sub(take)))
    }

    /// Chain evidence for a 32-byte constant, cached and archived.
    fn classify(&mut self, hex: &str) -> Result<TokenClass, MapError> {
        if let Some(c) = self.tokens.get(hex) {
            return Ok(c.clone());
        }
        let class = match self.source.token_info(hex) {
            Ok(info) if info.is_singleton() => TokenClass::Singleton,
            Ok(info) => TokenClass::Fungible(info.emission_amount),
            // Only a chain "no such token" is evidence of absence. A source
            // that cannot answer at all is a gap in the *source*, and the map
            // says so rather than recording `notAToken` on evidence it never
            // got — classification is the one query the map cannot do without.
            Err(SourceError::NotFound(_)) => TokenClass::NotAToken,
            Err(e) => return Err(MapError::Source(e)),
        };
        self.tokens.insert(hex.to_string(), class.clone());
        Ok(class)
    }

    /// The boxes selected for a token id, cached. Truncation is recorded per
    /// token id, not only overall: omitting a holder can omit the box
    /// actually at risk.
    fn token_boxes(&mut self, token_id: &str) -> Result<Vec<ChainBox>, MapError> {
        if let Some(v) = self.token_boxes.get(token_id) {
            return Ok(v.clone());
        }
        let (selected, omitted) = self.select(self.opts.max_boxes_per_token, |off, lim| {
            self.source.boxes_by_token_id(token_id, off, lim)
        })?;
        if omitted > 0 {
            self.truncated
                .per_token
                .insert(token_id.to_string(), omitted);
        }
        self.token_boxes
            .insert(token_id.to_string(), selected.clone());
        Ok(selected)
    }

    /// The boxes whose `blake2b256(propositionBytes)` is `hash`, cached.
    ///
    /// The *second* way a 32-byte constant resolves. A source without such an
    /// index says [`SourceError::Unsupported`], and the map then leaves the
    /// reference to close against trees it already holds, or to stand as an
    /// unresolved edge — never as an absence.
    fn script_hash_boxes(&mut self, hash: &str) -> Result<Vec<ChainBox>, MapError> {
        if self.script_hash_unsupported {
            return Ok(Vec::new());
        }
        if let Some(v) = self.script_hash_boxes.get(hash) {
            return Ok(v.clone());
        }
        let selected = match self.select(self.opts.max_boxes_per_script_hash, |off, lim| {
            self.source.boxes_by_script_hash(hash, off, lim)
        }) {
            Ok((selected, omitted)) => {
                if omitted > 0 {
                    self.truncated
                        .per_script_hash
                        .insert(hash.to_string(), omitted);
                }
                selected
            }
            Err(SourceError::Unsupported(_)) => {
                self.script_hash_unsupported = true;
                return Ok(Vec::new());
            }
            Err(e) => return Err(MapError::Source(e)),
        };
        self.script_hash_boxes
            .insert(hash.to_string(), selected.clone());
        Ok(selected)
    }

    /// The initial frontier: seed → boxes, bounded and truncation-reported.
    fn seed_frontier(&mut self, seed: &Seed) -> Result<Vec<ChainBox>, MapError> {
        let (mut boxes, omitted) = match seed {
            Seed::TokenId(id) => {
                // Deliberately NOT cached as this token's selection: the
                // frontier cap and `max_boxes_per_token` are different caps,
                // and an edge to the seed token must obey the latter.
                self.select(self.opts.max_frontier, |off, lim| {
                    self.source.boxes_by_token_id(id, off, lim)
                })?
            }
            Seed::Address(addr) => self.select(self.opts.max_frontier, |off, lim| {
                self.source.boxes_by_address(addr, off, lim)
            })?,
            Seed::TransactionId(tx) => {
                let t = self.source.transaction(tx)?;
                let mut all = t.inputs;
                all.extend(t.outputs);
                all.sort_by(|a, b| a.order_key().cmp(&b.order_key()));
                all.dedup_by(|a, b| a.box_id == b.box_id);
                let n = all.len();
                let kept: Vec<ChainBox> = all.into_iter().take(self.opts.max_frontier).collect();
                let omitted = n.saturating_sub(kept.len());
                (kept, omitted)
            }
        };
        self.truncated.frontier = omitted;
        boxes.sort_by(|a, b| a.order_key().cmp(&b.order_key()));
        if boxes.is_empty() {
            return Err(MapError::Seed(format!(
                "{} `{}` resolves to no unspent box at this height",
                seed.kind(),
                seed.value()
            )));
        }
        Ok(boxes)
    }
}

/// Decompile a box's tree and read its box references.
fn build_node(b: &ChainBox, depth: u32, testnet: bool) -> MapNode {
    let mut node = MapNode {
        chain_box: b.clone(),
        tree_hash: String::new(),
        depth,
        refs: TreeRefs::default(),
        complete: false,
        tree_error: None,
        nft: None,
        role: Role::Unknown,
    };
    let bytes = match hex::decode(&b.ergo_tree) {
        Ok(v) => v,
        Err(e) => {
            node.tree_error = Some(format!("ergoTree is not hex: {e}"));
            return node;
        }
    };
    node.tree_hash = hex::encode(ergo_primitives::digest::blake2b256(&bytes).as_bytes());
    match crate::inspect::parse_tree(&bytes) {
        Ok(tree) => {
            let lifted = crate::lift_tree(&tree, testnet);
            node.complete = lifted.raw_placeholders == 0 && !lifted.truncated;
            node.refs = tree_refs(&lifted.node);
        }
        Err(e) => node.tree_error = Some(e.to_string()),
    }
    node
}

/// A box's protocol NFT: `tokens(0)` when that qualifies (the singleton
/// convention every deployed set here follows), otherwise the lowest-indexed
/// token that does. Unrelated tokens at any index are ignored for identity.
fn protocol_nft_of(b: &ChainBox, protocol_nfts: &BTreeSet<String>) -> Option<String> {
    b.tokens
        .iter()
        .find(|t| protocol_nfts.contains(&t.id))
        .map(|t| t.id.clone())
}

// ── edges ────────────────────────────────────────────────────────────────────

/// Every edge a tree states outright: NFT bindings, script-hash bindings, the
/// self-successor, and the unpinned sites that resolve to nothing.
fn build_edges(
    nodes: &BTreeMap<String, MapNode>,
    token_boxes: &BTreeMap<String, Vec<ChainBox>>,
    tokens: &BTreeMap<String, TokenClass>,
) -> Vec<Edge> {
    let by_tree_hash: BTreeMap<&str, Vec<&str>> =
        nodes
            .values()
            .fold(BTreeMap::new(), |mut acc: BTreeMap<&str, Vec<&str>>, n| {
                acc.entry(n.tree_hash.as_str())
                    .or_default()
                    .push(n.chain_box.box_id.as_str());
                acc
            });

    let mut out = Vec::new();
    for node in nodes.values() {
        let from = node.chain_box.box_id.clone();
        for r in node.refs.slots.values() {
            if r.is_self {
                continue;
            }
            let mut resolved = false;

            for c in &r.nft_constants {
                let singleton = matches!(tokens.get(c), Some(TokenClass::Singleton));
                let targets: Vec<Target> = token_boxes
                    .get(c)
                    .map(|v| {
                        v.iter()
                            .filter(|b| nodes.contains_key(&b.box_id))
                            .map(|b| Target::Node(b.box_id.clone()))
                            .collect()
                    })
                    .unwrap_or_default();
                let targets = if targets.is_empty() {
                    vec![Target::Unresolved(c.clone())]
                } else {
                    targets
                };
                for t in targets {
                    resolved = true;
                    push_edge(&mut out, &from, t, Binding::Nft, Some(singleton), r);
                }
            }

            for h in &r.script_hashes {
                // Closed against the trees the map holds. The walk has
                // already asked the source for boxes with this script, where
                // the source has such an index at all, so anything it could
                // find is already a node here.
                let mut targets: Vec<Target> = by_tree_hash
                    .get(h.as_str())
                    .map(|ids| ids.iter().map(|i| Target::Node((*i).to_string())).collect())
                    .unwrap_or_default();
                if targets.is_empty() {
                    targets.push(Target::Unresolved(h.clone()));
                }
                for t in targets {
                    resolved = true;
                    push_edge(&mut out, &from, t, Binding::ScriptHash, None, r);
                }
            }

            if r.self_successor {
                resolved = true;
                push_edge(
                    &mut out,
                    &from,
                    Target::Node(from.clone()),
                    Binding::SelfSuccessor,
                    None,
                    r,
                );
            }

            // An unpinned site is still a reference. It may yet be paired with
            // a counterparty by `add_pairing_edges`; if it is not, it stays in
            // the output as unresolved rather than vanishing.
            if !resolved {
                push_edge(
                    &mut out,
                    &from,
                    Target::Unresolved(r.key.clone()),
                    if r.data_input {
                        Binding::DataInput
                    } else {
                        Binding::Positional
                    },
                    None,
                    r,
                );
            }
        }
    }
    out
}

fn push_edge(
    out: &mut Vec<Edge>,
    from: &str,
    to: Target,
    binding: Binding,
    singleton: Option<bool>,
    r: &BoxRef,
) {
    // A data-input site gets a `data-input` edge *in addition to* its
    // identity binding: the USE oracle is both pinned by NFT and read as a
    // data input, and the role rules need to see both.
    if r.data_input && binding != Binding::DataInput {
        out.push(Edge {
            from: from.to_string(),
            to: to.clone(),
            binding: Binding::DataInput,
            singleton: None,
            covers: r.covers.clone(),
            value_math: r.value_math,
            site: r.key.clone(),
            node_id: r.node_id,
        });
    }
    out.push(Edge {
        from: from.to_string(),
        to,
        binding,
        singleton,
        covers: r.covers.clone(),
        value_math: r.value_math,
        site: r.key.clone(),
        node_id: r.node_id,
    });
}

/// Pair an unpinned site with the counterparty the protocol clearly intends.
///
/// A tree that reads `INPUTS(0)` and checks nothing names no target — that is
/// the whole problem. The graph names it: if P pins A by A's protocol NFT,
/// then P and A are an intended pair, and A's unpinned sites are where the
/// pairing is *not* enforced from A's side. This is the reading that makes
/// "the swap references the pool by position" a statement about the pool.
///
/// The pairing is suppressed when A already pins P by NFT or script hash —
/// which is exactly how USE's `extract` differs from its `swap`.
fn add_pairing_edges(nodes: &BTreeMap<String, MapNode>, edges: &mut Vec<Edge>) {
    let pins: BTreeSet<(&str, &str)> = edges
        .iter()
        .filter(|e| matches!(e.binding, Binding::Nft | Binding::ScriptHash))
        .filter_map(|e| match &e.to {
            Target::Node(id) if id != &e.from => Some((e.from.as_str(), id.as_str())),
            _ => None,
        })
        .collect();

    let mut extra = Vec::new();
    for (a_id, a) in nodes {
        // Only a box with a protocol NFT can be the A of a pairing: without
        // one, "P names A's identity" is not something P could have done, and
        // any pin P happens to have on A is a coincidence of selection.
        if a.nft.is_none() {
            continue;
        }
        // Sites A leaves open, worst first: a site whose reserves drive value
        // maths is the one worth attributing.
        let mut open: Vec<&BoxRef> = a
            .refs
            .slots
            .values()
            .filter(|r| !r.is_self && !r.pinned())
            .collect();
        open.sort_by_key(|r| (!r.value_math, r.key.clone()));
        let Some(site) = open.first() else {
            continue;
        };
        for p_id in nodes.keys() {
            if p_id == a_id {
                continue;
            }
            // P pins A by NFT — the intended pairing.
            if !pins.contains(&(p_id.as_str(), a_id.as_str())) {
                continue;
            }
            // A enforces it from its side: nothing to report.
            if pins.contains(&(a_id.as_str(), p_id.as_str())) {
                continue;
            }
            extra.push(Edge {
                from: a_id.clone(),
                to: Target::Node(p_id.clone()),
                binding: if site.data_input {
                    Binding::DataInput
                } else {
                    Binding::Positional
                },
                singleton: None,
                covers: site.covers.clone(),
                value_math: site.value_math,
                site: site.key.clone(),
                node_id: site.node_id,
            });
        }
    }
    // The site now has a named counterparty; drop the unresolved placeholder
    // that stood in for it.
    let paired: BTreeSet<(String, String)> = extra
        .iter()
        .map(|e| (e.from.clone(), e.site.clone()))
        .collect();
    edges.retain(|e| {
        !(matches!(&e.to, Target::Unresolved(u) if u == &e.site)
            && paired.contains(&(e.from.clone(), e.site.clone())))
    });
    edges.extend(extra);
}

// ── roles ────────────────────────────────────────────────────────────────────

fn assign_roles(nodes: &mut BTreeMap<String, MapNode>, edges: &[Edge]) {
    let mut data_input_in: BTreeSet<&str> = BTreeSet::new();
    let mut positional_in: BTreeSet<&str> = BTreeSet::new();
    let mut value_math_in: BTreeSet<&str> = BTreeSet::new();
    for e in edges {
        let Target::Node(to) = &e.to else { continue };
        if to == &e.from {
            continue;
        }
        match e.binding {
            Binding::DataInput => {
                data_input_in.insert(to.as_str());
            }
            Binding::Positional => {
                positional_in.insert(to.as_str());
            }
            _ => {}
        }
        if e.value_math {
            value_math_in.insert(to.as_str());
        }
    }
    let roles: Vec<(String, Role)> = nodes
        .values()
        .map(|n| {
            let id = n.chain_box.box_id.as_str();
            let has_nft = n.nft.is_some();
            let carries = n.chain_box.value > COMPANION_MAX_VALUE
                || n.chain_box
                    .tokens
                    .iter()
                    .any(|t| Some(&t.id) != n.nft.as_ref());
            // Read for its data rather than spent. A data-input edge is
            // *sufficient* evidence, not a requirement that no other binding
            // exists — the USE oracle is pinned by NFT and read as a data
            // input both. What disqualifies a box is being read by
            // **position** by somebody: whatever else it is, a box a
            // contract picks out of a slot and does maths on is the box at
            // risk, and the hunt needs it labelled `protected`.
            let external = data_input_in.contains(id) && !positional_in.contains(id);
            let role = if external {
                Role::External
            } else if has_nft && carries && value_math_in.contains(id) {
                Role::Protected
            } else if has_nft && !carries {
                Role::Companion
            } else {
                Role::Unknown
            };
            (n.chain_box.box_id.clone(), role)
        })
        .collect();
    for (id, role) in roles {
        if let Some(n) = nodes.get_mut(&id) {
            n.role = role;
        }
    }
}

// ── set-level findings ───────────────────────────────────────────────────────

/// The graph form of `unbound-box-reserves`.
pub const LINT_UNBOUND_SET: &str = "set-unbound-box-reserves";
/// One side enforces the pairing, the other does not.
pub const LINT_ASYMMETRY: &str = "set-asymmetric-binding";

fn set_findings(nodes: &BTreeMap<String, MapNode>, edges: &[Edge]) -> Vec<SetFinding> {
    let mut out = Vec::new();
    for e in edges {
        let Target::Node(to) = &e.to else { continue };
        if to == &e.from {
            continue;
        }
        if e.binding == Binding::Positional && e.value_math {
            let Some(target) = nodes.get(to) else {
                continue;
            };
            if target.role != Role::Protected {
                continue;
            }
            out.push(SetFinding {
                finding: Finding {
                    lint: LINT_UNBOUND_SET,
                    severity: Severity::High,
                    node_id: e.node_id,
                    ir_id: None,
                    message: format!(
                        "reads {} by position and does value maths on it, but that slot is where \
                         the protected box {} sits — a box holding {} nanoERG and {} token(s). \
                         Nothing in this tree says which box goes there, so a transaction can \
                         place a decoy. Bind it by its NFT.",
                        e.site,
                        to,
                        target.chain_box.value,
                        target.chain_box.tokens.len()
                    ),
                    snippet: format!("{} -> {to} ({})", e.site, e.binding.as_str()),
                },
                from: e.from.clone(),
                to: e.to.clone(),
            });
        }
    }

    // Asymmetry: A pins B by NFT while B pins A only positionally.
    let nft_pins: BTreeSet<(&str, &str)> = edges
        .iter()
        .filter(|e| e.binding == Binding::Nft)
        .filter_map(|e| match &e.to {
            Target::Node(id) if id != &e.from => Some((e.from.as_str(), id.as_str())),
            _ => None,
        })
        .collect();
    for e in edges {
        if e.binding != Binding::Positional {
            continue;
        }
        let Target::Node(to) = &e.to else { continue };
        if !nft_pins.contains(&(to.as_str(), e.from.as_str())) {
            continue;
        }
        out.push(SetFinding {
            finding: Finding {
                lint: LINT_ASYMMETRY,
                severity: Severity::Medium,
                node_id: e.node_id,
                ir_id: None,
                message: format!(
                    "{to} pins this box by NFT, but this box refers back to it only by position \
                     ({}). The authors intended a pairing that one side does not enforce.",
                    e.site
                ),
                snippet: format!("{} -> {to} (positional; reverse edge is nft)", e.site),
            },
            from: e.from.clone(),
            to: e.to.clone(),
        });
    }
    out
}
