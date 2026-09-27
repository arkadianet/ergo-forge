//! The holdout harness: a bounded, honest measurement of the spend hunt on a
//! namespace it was not built from.
//!
//! Three gates, in order:
//!
//! 1. **The manifest is what it says.** Every contract's bytes match their
//!    recorded digest, every pair is one mechanical `find` → `replace` applied
//!    to its control, the compiled tree matches its recorded digest, and the
//!    cap policy is positive and equal to the instrument's own published caps
//!    and probe composition. A rate measured under different caps is a
//!    different rate, so the caps are checked rather than trusted.
//! 2. **The shadow model agrees with every recorded run.** [`shadow_model`]
//!    independently re-derives the bookkeeping relations between each
//!    aggregate verdict and its probe records, in both directions, and reports
//!    divergence. It is `static`/`preflight`, it can never mark a result
//!    node-validated, and it carries a
//!    [`Refusal`](shadow_model::Refusal) explaining why it does not decide
//!    validity. A divergence here is an instrument or harness bug, not a
//!    finding.
//! 3. **Exposure and the denominator are reported, and the denominator decides
//!    the rate.** [`credit`] derives the discovery-eligible denominator from
//!    two facts — a row's declared `authoring`, and a *measured* text-overlap
//!    scan of the visible corpus — and returns **no rate at all** when that
//!    denominator is zero. The scan compares clauses, not whole lines, so a
//!    corpus file that reuses one conjunct of a fixture is an overlap; it
//!    excludes the namespace and this manifest by construction; and its matcher
//!    is checked against a known overlap so that a scan which can match nothing
//!    cannot read as a clean corpus. The harness also refuses a manifest whose
//!    `independent` row is contradicted by a measured overlap. This namespace
//!    declares every row `instrument-visible`, so the honest outcome is:
//!    expectations reproduce, and the discovery rate is `null` over a zero
//!    denominator. Reproducing an expectation written while watching the
//!    instrument is *self-consistency*; it is reported under that name and never
//!    as detection.
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use ergo_sandbox::hunt::{hunt, HuntOptions, MAX_BOXES, MAX_PROBES};
use ergo_sandbox::shadow_model;
use ergo_sandbox::{compile, ScenarioBox, DEFAULT_COST_LIMIT};
use ergo_ser::address::NetworkPrefix;
use serde::Deserialize;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

use shadow_model::{
    DivergenceClass, RecordedEval, ShadowModel, ShadowPolicy, Signal, AGGREGATE_CLASSES, AUTHORITY,
    MAX_OBSERVED_PROBES, MAX_PROBE_ERRORING_READS, MAX_REGISTER_READS, MAX_RESIDUALS,
    MAX_RESIDUAL_TEXT, MAX_TEXT, METHOD, NOT_EVALUATED, PROBE_VERDICTS, SEVERITY_MEANING,
};

/// The manifest, as bytes.
const MANIFEST: &str = include_str!("../../examples/mutants/holdout.json");
/// Where that manifest lives, relative to the workspace root. The manifest
/// declares itself in its own `excluded` list, and the harness excludes it again
/// on top: a row's exposure must not be decidable by a document that could
/// simply leave its own path out, so the harness does not rely on that line.
const MANIFEST_PATH: &str = "examples/mutants/holdout.json";

/// The message behind a clean row the credit test needs. A row with no measured
/// overlap is the only kind this namespace may offer the credit path, so when
/// the corpus reaches every row that demonstration is gone — and the honest
/// move is to say so rather than to declare an overlapping row clean.
const NO_CLEAN_ROW: &str =
    "no holdout row is free of measured corpus overlap any more, so this namespace can no \
     longer demonstrate the credit path. Add a clause-distinct row, or re-scope the scan roots \
     deliberately; never mark an overlapping row clean to make this pass";

// ─────────────────────────────────────────────────────────────────────────────
// the manifest
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Deserialize, Clone)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Manifest {
    schema_version: u32,
    namespace: String,
    version: u32,
    method: String,
    authority: String,
    node_validated: bool,
    catalogue_claim: String,
    note: String,
    independence: Independence,
    caps: Caps,
    shadow_policy: ShadowPolicy,
    exposure_scan: ScanSpec,
    pairs: Vec<Pair>,
    items: Vec<Item>,
}

#[derive(Deserialize, Clone)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Independence {
    rule: String,
    authoring_values: BTreeMap<String, String>,
}

#[derive(Deserialize, Clone)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Caps {
    height: u32,
    network: String,
    tree_version: u8,
    max_probes: usize,
    max_boxes_per_collection: usize,
    block_cost_limit: u64,
    probe_set: Vec<String>,
    self_box_default: String,
}

#[derive(Deserialize, Clone)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ScanSpec {
    roots: Vec<String>,
    excluded: Vec<String>,
    extensions: Vec<String>,
    max_files: usize,
    max_bytes: u64,
    min_clause_chars: usize,
}

#[derive(Deserialize, Clone)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Pair {
    id: String,
    class: String,
    operator: String,
    control_path: String,
    counterexample_path: String,
    diff: Diff,
    basis: String,
    catalogue_vector: Option<String>,
    instrument_limit: Option<String>,
}

#[derive(Deserialize, Clone)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Diff {
    find: String,
    replace: String,
}

#[derive(Deserialize, Clone)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Item {
    id: String,
    pair: String,
    role: String,
    path: String,
    sha256: String,
    tree_sha256: String,
    self_box: Option<Value>,
    expected_verdict: String,
    expected_basis: String,
    authoring: String,
}

fn manifest() -> Manifest {
    serde_json::from_str(MANIFEST).expect("holdout manifest parses")
}

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("workspace parent")
        .to_path_buf()
}

fn network(prefix: &str) -> NetworkPrefix {
    match prefix {
        "mainnet" => NetworkPrefix::Mainnet,
        "testnet" => NetworkPrefix::Testnet,
        other => panic!("unknown network {other}"),
    }
}

fn digest(bytes: &[u8]) -> String {
    hex::encode(Sha256::digest(bytes))
}

// ─────────────────────────────────────────────────────────────────────────────
// one recorded run
// ─────────────────────────────────────────────────────────────────────────────

/// What the harness observed for one item, and the model's opinion of it.
struct Observed {
    id: String,
    role: String,
    pair: String,
    expected: String,
    verdict: String,
    reproduced: bool,
    synthetic: bool,
    record: RecordedEval,
    shadow: shadow_model::ShadowReport,
}

fn observe(item: &Item, caps: &Caps, policy: &ShadowPolicy) -> Observed {
    let path = root().join(&item.path);
    let bytes = std::fs::read(&path).unwrap_or_else(|e| panic!("{}: {e}", item.path));
    assert_eq!(
        digest(&bytes),
        item.sha256,
        "{}: the recorded source digest does not match the file. Re-record deliberately; \
         never edit the fixture to fit the run",
        item.id
    );
    let source = String::from_utf8(bytes).expect("holdout sources are UTF-8");
    let compiled = compile::compile_source(&source, caps.tree_version, network(&caps.network))
        .unwrap_or_else(|e| panic!("{}: does not compile: {e:?}", item.id));
    assert_eq!(
        digest(&compiled.tree_bytes),
        item.tree_sha256,
        "{}: the compiled tree changed — the pinned compiler moved. Re-record deliberately",
        item.id
    );

    let self_box: Option<ScenarioBox> = item
        .self_box
        .clone()
        .map(|v| serde_json::from_value(v).unwrap_or_else(|e| panic!("{}: selfBox: {e}", item.id)));
    let opts = HuntOptions {
        height: Some(caps.height),
        self_box,
        network: Some(network(&caps.network)),
        data_inputs: Vec::new(),
    };
    let report = hunt(&compiled.tree_bytes, &opts)
        .unwrap_or_else(|e| panic!("{}: hunt error: {e}", item.id));

    // The caps are part of the measurement: the observed run must have used the
    // declared budget, composition and cost limit, and must have filled it.
    let json: Value =
        serde_json::to_value(&report).expect("the hunt serialises; the model reads that shape");
    let observed_caps = &json["caps"];
    assert_eq!(
        observed_caps["maxProbes"], caps.max_probes,
        "{}: the observed probe budget differs from the declared policy",
        item.id
    );
    assert_eq!(
        observed_caps["maxBoxesPerCollection"], caps.max_boxes_per_collection,
        "{}: the observed positional cap differs from the declared policy",
        item.id
    );
    assert_eq!(
        observed_caps["blockCostLimit"], caps.block_cost_limit,
        "{}: the observed cost limit differs from the declared policy",
        item.id
    );
    assert_eq!(
        json["probeSet"],
        json!(caps.probe_set),
        "{}: the observed probe composition differs from the declared policy",
        item.id
    );
    assert_eq!(
        json["probes"].as_array().map(Vec::len),
        Some(caps.max_probes),
        "{}: the run did not fill the declared probe budget",
        item.id
    );
    assert_eq!(
        json["selfSynthetic"].as_bool(),
        Some(item.self_box.is_none()),
        "{}: the recorded SELF presence does not match the run",
        item.id
    );

    for field in [
        "method",
        "verdict",
        "nodeValidated",
        "selfSynthetic",
        "truncated",
        "registerReads",
        "residuals",
        "probes",
        "observation",
    ] {
        assert!(
            json.get(field).is_some(),
            "{}: producer field `{field}` disappeared; the shadow model would silently weaken",
            item.id
        );
    }
    for probe in json["probes"].as_array().expect("probes array") {
        for field in [
            "kind",
            "output",
            "verdict",
            "erroringReads",
            "cost",
            "costExhausted",
        ] {
            assert!(
                probe.get(field).is_some(),
                "{}: producer probe field `{field}` disappeared; the shadow model would silently weaken",
                item.id
            );
        }
    }

    // The model reads the instrument's own output, not a summary of it.
    let record: RecordedEval =
        serde_json::from_value(json.clone()).unwrap_or_else(|e| panic!("{}: {e}", item.id));
    assert!(
        record.probes.len() <= MAX_OBSERVED_PROBES,
        "{}: the run carries more probes than the model will read",
        item.id
    );
    let shadow = ShadowModel::new(policy.clone()).observe(&item.id, &record);
    let verdict = json["verdict"]
        .as_str()
        .expect("the hunt's verdict token")
        .to_string();
    Observed {
        id: item.id.clone(),
        role: item.role.clone(),
        pair: item.pair.clone(),
        expected: item.expected_verdict.clone(),
        reproduced: verdict == item.expected_verdict,
        synthetic: record.self_synthetic,
        record,
        shadow,
        verdict,
    }
}

fn observe_all(m: &Manifest) -> Vec<Observed> {
    m.items
        .iter()
        .map(|i| observe(i, &m.caps, &m.shadow_policy))
        .collect()
}

// ─────────────────────────────────────────────────────────────────────────────
// exposure: what the instrument could already have been tuned on
// ─────────────────────────────────────────────────────────────────────────────

/// The measured text overlap between the namespace and the visible corpus.
/// Every number here is measured from files; none of them is claimed by the
/// manifest.
#[derive(Default)]
struct Exposure {
    files: usize,
    bytes: u64,
    truncated: bool,
    /// item id → corpus paths whose whole content normalises to the item's.
    verbatim: BTreeMap<String, Vec<String>>,
    /// item id → corpus paths containing a clause of the item's.
    clauses: BTreeMap<String, Vec<String>>,
}

/// One corpus file: where it was read from, and its normalised text.
struct CorpusFile {
    path: String,
    text: String,
}

impl Exposure {
    fn overlaps(&self, id: &str) -> bool {
        self.verbatim.contains_key(id) || self.clauses.contains_key(id)
    }

    fn paths(&self, id: &str) -> Vec<String> {
        self.verbatim
            .get(id)
            .into_iter()
            .chain(self.clauses.get(id))
            .flatten()
            .cloned()
            .collect()
    }

    /// Measure one item against the corpus, recording where it was found.
    ///
    /// Split out of the filesystem walk so the *matcher* can be tested against a
    /// known overlap: a scan that found nothing because it can match nothing
    /// prints exactly like a clean corpus, and only a positive control tells
    /// the two apart.
    fn measure(&mut self, id: &str, source: &str, min_clause_chars: usize, corpus: &[CorpusFile]) {
        let whole = normalise(source);
        let clauses = clauses_of(source, min_clause_chars);
        let mut verbatim = Vec::new();
        let mut shared = Vec::new();
        for file in corpus {
            if file.text == whole {
                verbatim.push(file.path.clone());
            } else if clauses.iter().any(|c| file.text.contains(c.as_str())) {
                shared.push(file.path.clone());
            }
        }
        if !verbatim.is_empty() {
            self.verbatim.insert(id.to_string(), verbatim);
        }
        if !shared.is_empty() {
            self.clauses.insert(id.to_string(), shared);
        }
    }

    fn report(&self) -> Value {
        json!({
            "filesScanned": self.files,
            "bytesScanned": self.bytes,
            "scanTruncated": self.truncated,
            "verbatimOverlaps": self.verbatim,
            "clauseOverlaps": self.clauses,
            "note": "text overlap measured against the declared roots and extensions, minus the \
                     declared exclusions and this harness's own manifest. A clause is a line or a \
                     conjunct of one, split on newlines, &&, || and ';', so a corpus file that \
                     reuses a single conjunct of a holdout fixture counts as an overlap. A file \
                     past the declared cap was not read, so a clean report means 'no overlap \
                     found in what was read'",
        })
    }
}

/// Collapse whitespace, so a reformatted copy of a clause still reads as the
/// same clause.
fn normalise(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut space = false;
    for ch in text.chars() {
        if ch.is_whitespace() {
            space = true;
            continue;
        }
        if space && !out.is_empty() {
            out.push(' ');
        }
        space = false;
        out.push(ch);
    }
    out
}

/// A record's identifying clauses.
///
/// A line is not a clause. An ErgoScript condition is a conjunction, and a
/// holdout fixture is usually one line holding three of them, so a whole-line
/// test only ever compares a fixture against a byte-identical copy of itself:
/// the successor-binding idiom alone is reused across the visible corpus, and a
/// matcher that cannot see a shared conjunct reports zero overlap on a corpus an
/// author could plainly have been shown. The split is therefore on newlines and
/// on the boolean/statement separators `&&`, `||` and `;`, and the fragments are
/// kept only when they are long enough to mean something — a shorter one is a
/// token any two ErgoScript files share.
fn clauses_of(source: &str, min_chars: usize) -> Vec<String> {
    source
        .split(['\n', '&', '|', ';'])
        .map(normalise)
        .filter(|clause| clause.chars().count() >= min_chars)
        .collect()
}

/// Keep a failure message readable when the field that broke a bound is itself
/// megabytes long: quote the head, not the whole field.
fn trunc_for_message(text: &str) -> String {
    const HEAD: usize = 64;
    if text.chars().count() <= HEAD {
        return text.to_string();
    }
    let head: String = text.chars().take(HEAD).collect();
    format!("{head}… ({} chars)", text.chars().count())
}

fn collect_files(dir: &Path, extensions: &BTreeSet<&str>, out: &mut Vec<PathBuf>) {
    let mut entries: Vec<PathBuf> = std::fs::read_dir(dir)
        .unwrap_or_else(|e| panic!("{}: {e}", dir.display()))
        .map(|e| e.expect("directory entry").path())
        .collect();
    entries.sort();
    for path in entries {
        if path.is_dir() {
            collect_files(&path, extensions, out);
        } else if path
            .extension()
            .and_then(|e| e.to_str())
            .is_some_and(|e| extensions.contains(e))
        {
            out.push(path);
        }
    }
}

/// Everything the scan refuses to read as corpus material: the manifest's own
/// declaration, plus the harness's own exclusions. A namespace and the document
/// that describes it are not evidence about the visible corpus, and the second
/// half is applied by the harness so the first half cannot be edited away.
fn excluded_paths(m: &Manifest, harness: &[&str]) -> Vec<PathBuf> {
    let mut out: Vec<PathBuf> = m
        .exposure_scan
        .excluded
        .iter()
        .map(PathBuf::from)
        .chain(harness.iter().map(|p| PathBuf::from(*p)))
        .collect();
    out.sort();
    out.dedup();
    out
}

/// The files the scan would read, given the exclusions to apply. `harness` is a
/// parameter rather than a constant so a test can measure what the harness's own
/// exclusions are worth, against what the manifest alone excludes.
fn scan_candidates(m: &Manifest, harness: &[&str]) -> Vec<PathBuf> {
    let spec = &m.exposure_scan;
    let base = root();
    // `Path::extension` yields `es`, the manifest spells it `.es`.
    let extensions: BTreeSet<&str> = spec
        .extensions
        .iter()
        .map(|e| e.trim_start_matches('.'))
        .collect();
    assert!(
        extensions.iter().all(|e| !e.is_empty()),
        "every declared extension must name something"
    );
    let excluded = excluded_paths(m, harness);
    let mut candidates: Vec<PathBuf> = Vec::new();
    for dir in &spec.roots {
        collect_files(&base.join(dir), &extensions, &mut candidates);
    }
    // The namespace's own files are not corpus material: a holdout that
    // overlaps itself measures nothing.
    candidates.retain(|p| {
        let rel = p.strip_prefix(&base).unwrap_or(p);
        !excluded.iter().any(|x| rel.starts_with(x))
    });
    candidates
}

fn scan_exposure(m: &Manifest) -> Exposure {
    let spec = &m.exposure_scan;
    let base = root();
    // This harness's own manifest is excluded here, not by the manifest.
    let candidates = scan_candidates(m, &[MANIFEST_PATH]);
    assert!(
        !candidates.contains(&base.join(MANIFEST_PATH)),
        "the scan must not read the document that describes the rows it judges"
    );

    let mut exposure = Exposure::default();
    let mut corpus: Vec<CorpusFile> = Vec::new();
    for path in &candidates {
        if exposure.files >= spec.max_files || exposure.bytes >= spec.max_bytes {
            exposure.truncated = true;
            break;
        }
        let Ok(bytes) = std::fs::read(path) else {
            continue;
        };
        exposure.files += 1;
        exposure.bytes += bytes.len() as u64;
        corpus.push(CorpusFile {
            path: path
                .strip_prefix(&base)
                .unwrap_or(path)
                .display()
                .to_string(),
            text: normalise(&String::from_utf8_lossy(&bytes)),
        });
    }

    for item in &m.items {
        let source = std::fs::read_to_string(base.join(&item.path)).expect("item source");
        exposure.measure(&item.id, &source, spec.min_clause_chars, &corpus);
    }
    exposure
}

// ─────────────────────────────────────────────────────────────────────────────
// credit: the denominator decides the rate
// ─────────────────────────────────────────────────────────────────────────────

/// The measurement's own accounting. A rate is present only when the derived
/// denominator is non-empty, and it is never computed over anything else.
#[derive(Debug)]
struct Credit {
    items: usize,
    declared_independent: usize,
    declared_instrument_visible: usize,
    measured_verbatim_overlap: usize,
    measured_clause_overlap: usize,
    eligible: usize,
    detection_rate: Option<f64>,
    rate_reason: String,
    reproduced: usize,
    pairs: usize,
    separating_pairs: usize,
}

impl Credit {
    fn report(&self) -> Value {
        json!({
            "items": self.items,
            "declaredIndependent": self.declared_independent,
            "declaredInstrumentVisible": self.declared_instrument_visible,
            "measuredVerbatimOverlap": self.measured_verbatim_overlap,
            "measuredClauseOverlap": self.measured_clause_overlap,
            "discoveryCredit": {
                "eligibleDenominator": self.eligible,
                "detectionRate": self.detection_rate,
                "reason": self.rate_reason,
            },
            "selfConsistency": {
                "note": "recorded expectation vs this run — NOT discovery credit",
                "reproduced": self.reproduced,
                "of": self.items,
            },
            "discrimination": {
                "pairs": self.pairs,
                "separatingPairs": self.separating_pairs,
            },
        })
    }
}

/// Derive the discovery credit, or refuse.
///
/// * `independent` + a measured overlap is a **dishonest row**, not a weak one:
///   the manifest claims the author never saw the instrument while the visible
///   corpus already contains the contract. Refused.
/// * An `authoring` value outside the declared vocabulary is refused: the
///   vocabulary is a policy decision, not free text.
/// * No eligible item means **no rate**. A number over an empty denominator is
///   not a measurement, and the reason is reported so the zero is legible.
fn credit(
    items: &[Item],
    observed: &[Observed],
    exposure: &Exposure,
    m: &Manifest,
) -> Result<Credit, String> {
    let mut independent = 0usize;
    let mut visible = 0usize;
    for item in items {
        match item.authoring.as_str() {
            "independent" => independent += 1,
            "instrument-visible" => visible += 1,
            other => {
                return Err(format!(
                    "{}: unknown authoring value `{other}`. The manifest's independence rule \
                     names the vocabulary, so a new value must be added there and argued for",
                    item.id
                ))
            }
        }
        if item.authoring == "independent" && exposure.overlaps(&item.id) {
            return Err(format!(
                "{}: claims `independent`, but the measured scan finds it in {:?}. An \
                 independent row may not be a contract the instrument could have been tuned on",
                item.id,
                exposure.paths(&item.id)
            ));
        }
    }

    // Discrimination is DERIVED from the two observed verdicts of each pair, so
    // a pair that stopped separating cannot be described as separating.
    let mut separating = 0usize;
    for pair in &m.pairs {
        let rows: Vec<&Observed> = observed.iter().filter(|o| o.pair == pair.id).collect();
        if rows.len() != 2 {
            return Err(format!(
                "{}: a holdout pair must have exactly two recorded runs, saw {}",
                pair.id,
                rows.len()
            ));
        }
        if rows[0].verdict != rows[1].verdict {
            separating += 1;
        }
    }

    let eligible: Vec<&Item> = items
        .iter()
        .filter(|i| i.authoring == "independent" && !exposure.overlaps(&i.id))
        .collect();
    let reproduced = observed.iter().filter(|o| o.reproduced).count();
    let (detection_rate, rate_reason) = if eligible.is_empty() {
        (
            None,
            format!(
                "no eligible items: {visible} of {} rows are `instrument-visible` (authored \
                 from this instrument's own output, so reproducing them is self-consistency), \
                 and no row is `independent` with a clean measured scan",
                items.len()
            ),
        )
    } else {
        let hits = eligible
            .iter()
            .filter(|i| observed.iter().any(|o| o.id == i.id && o.reproduced))
            .count();
        (
            Some(hits as f64 / eligible.len() as f64),
            format!(
                "derived from {} eligible item(s): declared `independent` with no measured \
                 overlap against the visible corpus",
                eligible.len()
            ),
        )
    };

    Ok(Credit {
        items: items.len(),
        declared_independent: independent,
        declared_instrument_visible: visible,
        measured_verbatim_overlap: exposure.verbatim.len(),
        measured_clause_overlap: exposure.clauses.len(),
        eligible: eligible.len(),
        detection_rate,
        rate_reason,
        reproduced,
        pairs: m.pairs.len(),
        separating_pairs: separating,
    })
}

// ─────────────────────────────────────────────────────────────────────────────
// gates
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn the_manifest_is_what_it_says() {
    let m = manifest();
    assert_eq!(m.schema_version, 1);
    assert_eq!(m.namespace, "holdout");
    assert_eq!(m.version, 1);
    assert_eq!(m.method, "bounded-scenario-sampling");
    assert_eq!(m.authority, AUTHORITY);
    assert!(
        !m.node_validated,
        "the namespace may never claim node validation"
    );
    assert!(
        m.catalogue_claim.starts_with("none"),
        "the namespace adds no vector row"
    );
    assert!(!m.note.is_empty() && !m.independence.rule.is_empty());
    // The independence vocabulary is the one `credit` enforces.
    assert_eq!(m.independence.authoring_values.len(), 2);
    assert!(m
        .independence
        .authoring_values
        .keys()
        .any(|k| k == "independent"));
    assert!(m
        .independence
        .authoring_values
        .values()
        .all(|v| !v.is_empty()));

    // ── the cap policy: positive, and equal to the instrument's own ──
    let caps = &m.caps;
    assert!(caps.height > 0, "the declared height must be positive");
    assert!(matches!(caps.network.as_str(), "mainnet" | "testnet"));
    assert!(caps.max_probes > 0 && caps.block_cost_limit > 0);
    assert_eq!(
        caps.max_probes, MAX_PROBES,
        "the declared probe budget differs from the instrument's published cap"
    );
    assert_eq!(
        caps.max_boxes_per_collection, MAX_BOXES,
        "the declared positional cap differs from the instrument's published cap"
    );
    assert_eq!(
        caps.block_cost_limit, DEFAULT_COST_LIMIT,
        "the declared cost limit differs from the evaluator's published default"
    );
    assert!(
        !caps.probe_set.is_empty() && caps.probe_set.iter().all(|k| !k.is_empty()),
        "the probe composition is part of the measurement: a different set measures a \
         different thing"
    );
    assert!(!caps.self_box_default.is_empty());

    // ── the scan spec is bounded, so "no overlap found" is a statement ──
    let scan = &m.exposure_scan;
    assert!(!scan.roots.is_empty() && !scan.extensions.is_empty());
    assert!(
        !scan.excluded.is_empty(),
        "the namespace excludes itself from its own exposure scan"
    );
    assert!(scan.max_files > 0 && scan.max_bytes > 0);
    assert!(
        scan.min_clause_chars >= 16,
        "a clause shorter than this is not identifying"
    );
    // The namespace cannot measure itself, and neither can the manifest: a row
    // whose exposure is decided by a file that describes the row is not
    // measured. The manifest is excluded by the harness regardless of what the
    // manifest declares, so a forgery cannot launder a row by omission.
    let excluded = excluded_paths(&m, &[MANIFEST_PATH]);
    assert!(
        excluded.contains(&PathBuf::from(MANIFEST_PATH)),
        "the harness must exclude its own manifest from its own exposure scan"
    );
    let under_exclusion = |path: &str| {
        let path = Path::new(path);
        excluded.iter().any(|x| path.starts_with(x))
    };
    for pair in &m.pairs {
        assert!(
            under_exclusion(&pair.control_path) && under_exclusion(&pair.counterexample_path),
            "{}: a holdout pair must lie outside its own exposure roots",
            pair.id
        );
    }
    for item in &m.items {
        assert!(
            under_exclusion(&item.path),
            "{}: a holdout row must lie outside its own exposure roots",
            item.id
        );
        assert!(
            !item.path.starts_with(MANIFEST_PATH),
            "{}: the manifest is not a row",
            item.id
        );
    }

    // ── pairs: one mechanical edit, and both halves on disk ──
    let mut pair_ids = BTreeSet::new();
    for pair in &m.pairs {
        assert!(pair_ids.insert(pair.id.clone()), "duplicate pair id");
        assert_eq!(pair.operator, "delete-clause");
        assert!(!pair.class.is_empty() && !pair.basis.is_empty());
        if let Some(limit) = &pair.instrument_limit {
            assert!(
                !limit.is_empty(),
                "{}: an instrument limit must be stated, not implied",
                pair.id
            );
        }
        assert_eq!(
            pair.catalogue_vector, None,
            "{}: a holdout pair is discrimination material, not a catalogue row",
            pair.id
        );
        let control = std::fs::read_to_string(root().join(&pair.control_path))
            .unwrap_or_else(|e| panic!("{}: {e}", pair.control_path));
        let counterexample = std::fs::read_to_string(root().join(&pair.counterexample_path))
            .unwrap_or_else(|e| panic!("{}: {e}", pair.counterexample_path));
        assert_eq!(
            control.matches(&pair.diff.find).count(),
            1,
            "{}: the recorded `find` must be unique in its control",
            pair.id
        );
        assert_eq!(
            control.replacen(&pair.diff.find, &pair.diff.replace, 1),
            counterexample,
            "{}: the pair is not one recorded replacement",
            pair.id
        );
    }

    // ── items: unique, paired, and every basis stated ──
    let mut ids = BTreeSet::new();
    for item in &m.items {
        assert!(ids.insert(item.id.clone()), "duplicate item id");
        assert!(matches!(item.role.as_str(), "control" | "counterexample"));
        assert!(pair_ids.contains(&item.pair), "{}: unknown pair", item.id);
        assert!(
            AGGREGATE_CLASSES
                .iter()
                .any(|(class, _)| *class == item.expected_verdict),
            "{}: expected verdict `{}` is outside the vocabulary the model can check",
            item.id,
            item.expected_verdict
        );
        assert!(
            !item.expected_basis.is_empty(),
            "{}: an expectation without a stated basis is a claim",
            item.id
        );
        assert_eq!(item.sha256.len(), 64);
        assert_eq!(item.tree_sha256.len(), 64);
    }
    assert_eq!(
        m.items.len(),
        m.pairs.len() * 2,
        "every pair carries two runs"
    );
    for pair in &m.pairs {
        let rows: Vec<&Item> = m.items.iter().filter(|i| i.pair == pair.id).collect();
        let control = rows
            .iter()
            .find(|i| i.role == "control")
            .unwrap_or_else(|| panic!("{}: exactly one control", pair.id));
        let counterexample = rows
            .iter()
            .find(|i| i.role == "counterexample")
            .unwrap_or_else(|| panic!("{}: exactly one counterexample", pair.id));
        assert_eq!(rows.len(), 2, "{}: exactly two runs", pair.id);
        assert_eq!(control.path, pair.control_path);
        assert_eq!(counterexample.path, pair.counterexample_path);
    }
}

#[test]
fn every_recorded_run_is_consistent_with_the_shadow_model() {
    let m = manifest();
    let observed = observe_all(&m);
    assert_eq!(observed.len(), m.items.len());
    for o in &observed {
        let report = &o.shadow;
        assert_eq!(
            report.signal,
            Signal::Consistent,
            "{}: the shadow model diverged from the recorded run: {:?}",
            o.id,
            report.divergences
        );
        // The boundary, on every report: static/preflight, never node-validated,
        // never prose, with the refusal attached and the blind list named.
        let json = serde_json::to_value(report).expect("the report serialises");
        assert_eq!(json["method"], METHOD);
        assert_eq!(json["authority"], AUTHORITY);
        assert_eq!(json["nodeValidated"], false);
        assert_eq!(json["severityMeaning"], SEVERITY_MEANING);
        assert_eq!(json["proseUsed"], false);
        assert_eq!(json["refusal"]["reason"], "consensus-duplication");
        assert_eq!(json["notEvaluated"], json!(NOT_EVALUATED));
        assert!(
            !report.evaluated.is_empty(),
            "{}: a consistent report must say which rules ran",
            o.id
        );
        assert!(
            report.unchecked.is_empty(),
            "{}: nothing was left unchecked",
            o.id
        );
        let rendered = json.to_string();
        for forbidden in ["acceptedExecution", "confirmedViolation"] {
            assert!(
                !rendered.contains(forbidden),
                "{}: a shadow report may never name an execution fact",
                o.id
            );
        }
        // The bounds the model enforces are the bounds these records are
        // actually inside: a consistent report is only consistent because the
        // model read the whole record, so a fixture that outgrew a cap would
        // show up here as a `recordTooLarge` finding rather than as a verdict.
        let max_text = m.shadow_policy.max_text();
        assert!(
            o.record.register_reads.len() <= MAX_REGISTER_READS
                && o.record.residuals.len() <= MAX_RESIDUALS,
            "{}: the recorded run is past a count cap the model enforces",
            o.id
        );
        for text in [&o.record.verdict, &o.record.method, &o.record.observation]
            .into_iter()
            .chain(o.record.register_reads.iter())
            .chain(o.record.residuals.iter())
        {
            assert!(
                text.chars().count() <= max_text,
                "{}: `{}` is past the policy's text cap of {max_text}",
                o.id,
                trunc_for_message(text)
            );
        }
        // Every token this run produced is one the model can interpret — a
        // report that cannot read its own input is not evidence of anything.
        for probe in &o.record.probes {
            assert!(
                PROBE_VERDICTS.contains(&probe.verdict.as_str()),
                "{}: probe token `{}` is outside the model's vocabulary",
                o.id,
                probe.verdict
            );
            assert!(
                probe.erroring_reads.len() <= MAX_PROBE_ERRORING_READS,
                "{}: probe `{}` discloses more reads than the model reads",
                o.id,
                probe.kind
            );
            for text in [&probe.kind, &probe.output, &probe.verdict]
                .into_iter()
                .chain(probe.reduced_to.iter())
                .chain(probe.erroring_reads.iter())
            {
                assert!(
                    text.chars().count() <= max_text,
                    "{}: probe field `{}` is past the policy's text cap of {max_text}",
                    o.id,
                    trunc_for_message(text)
                );
            }
        }
        assert!(
            AGGREGATE_CLASSES
                .iter()
                .any(|(class, _)| *class == o.record.verdict),
            "{}: aggregate `{}` is outside the model's vocabulary",
            o.id,
            o.record.verdict
        );
    }
    // Both SELF modes are represented, so neither path is untested by this
    // namespace: a self box is part of the measurement, and saying so is the
    // difference between a recorded run and a defaulted one.
    assert!(observed.iter().any(|o| o.synthetic));
    assert!(observed.iter().any(|o| !o.synthetic));
}

/// The three facts the evidence boundary owns are uninhabited types here: the
/// module has no constructor for them, so it cannot report one. This test pins
/// that they still exist and are named — the proof itself is a reviewable
/// property of `shadow_model.rs`, which a runtime assertion cannot establish.
#[test]
fn the_shadow_model_holds_no_execution_fact() {
    fn named_only<T>() {}
    named_only::<shadow_model::NodeValidation>();
    named_only::<shadow_model::AcceptedExecution>();
    named_only::<shadow_model::ConfirmedViolation>();

    let refusal = shadow_model::refusal();
    assert_eq!(refusal.reason, "consensus-duplication");
    assert!(!refusal.refused.is_empty() && !refusal.text.is_empty());
    assert!(refusal.instead.contains("consistency"));
    assert_eq!(
        serde_json::to_value(refusal).expect("the refusal serialises")["reason"],
        "consensus-duplication"
    );

    // The bounds the model enforces on its input, checked where they are
    // decided: a bound that is zero or a detail cap above the field cap would
    // make the model's own claims untrue at compile time.
    const {
        assert!(MAX_OBSERVED_PROBES > 0 && MAX_REGISTER_READS > 0 && MAX_RESIDUALS > 0);
        assert!(MAX_PROBE_ERRORING_READS > 0);
        assert!(MAX_TEXT > 0);
        assert!(
            MAX_RESIDUAL_TEXT > 0 && MAX_RESIDUAL_TEXT < MAX_TEXT,
            "a finding's bounded detail is capped below a record's own text field"
        );
    }
    assert!(NOT_EVALUATED.contains(&"transaction-validity"));
}

/// Every finding must be bounded, and the bound is only a bound if it is
/// checked where findings exist: a report with no divergences would satisfy
/// this loop vacuously, so the helper refuses to run on one.
fn assert_findings_bounded(report: &shadow_model::ShadowReport, subject: &str) {
    assert!(
        !report.divergences.is_empty(),
        "{subject}: the finding assertions below would be vacuous"
    );
    for d in &report.divergences {
        assert!(
            !d.rule.is_empty() && !d.detail.is_empty(),
            "{subject}: a finding must name its rule and carry detail"
        );
        assert!(
            d.detail.chars().count() <= MAX_RESIDUAL_TEXT,
            "{subject}: rule `{}` emitted {} characters of detail, past the {MAX_RESIDUAL_TEXT} \
             the model claims",
            d.rule,
            d.detail.chars().count()
        );
        assert!(matches!(
            d.class,
            DivergenceClass::Violation | DivergenceClass::Refusal
        ));
    }
}

/// A record that contradicts itself is reported, not absorbed; a record that
/// claims node validation is a divergence, never an upgrade; and an
/// unreadable token leaves the dependent rules unchecked rather than passing.
#[test]
fn a_contradictory_record_is_reported_as_divergence() {
    let m = manifest();
    let real = observe_all(&m).remove(0).record;
    let model = ShadowModel::new(m.shadow_policy.clone());

    // An aggregate its own probes do not support.
    let mut broken = real.clone();
    broken.verdict = "spendableByAnyone".to_string();
    for probe in &mut broken.probes {
        probe.verdict = "fail".to_string();
    }
    let report = model.observe("broken-aggregate", &broken);
    assert_eq!(report.signal, Signal::Divergent);
    let kinds: Vec<&str> = report
        .divergences
        .iter()
        .map(|d| d.kind.wire_name())
        .collect();
    assert!(kinds.contains(&"aggregateUnsupportedByProbes"), "{kinds:?}");
    assert!(report
        .divergences
        .iter()
        .all(|d| d.class == DivergenceClass::Violation));
    assert_eq!(
        serde_json::to_value(&report).expect("serialises")["nodeValidated"],
        false
    );
    assert_findings_bounded(&report, "broken-aggregate");

    // A record that claims node validation.
    let mut claiming = real.clone();
    claiming.node_validated = true;
    let report = model.observe("claims-node-validation", &claiming);
    assert_eq!(report.signal, Signal::Divergent);
    assert!(report
        .divergences
        .iter()
        .any(|d| d.kind.wire_name() == "nodeValidationClaimed"));
    assert!(
        !report.node_validated,
        "the model never adopts the record's claim"
    );
    assert_findings_bounded(&report, "claims-node-validation");

    // A residual no needsProof probe produced.
    let mut orphan = real.clone();
    orphan.verdict = "requiresProof".to_string();
    orphan.residuals = vec!["ProveDlog(00)".to_string()];
    let report = model.observe("orphan-residual", &orphan);
    assert!(report
        .divergences
        .iter()
        .any(|d| d.kind.wire_name() == "residualWithoutProbe"));
    assert_findings_bounded(&report, "orphan-residual");

    // The bound on `detail` bites: a record carrying kilobytes of text still
    // yields findings inside the cap, with the truncation marker counted
    // inside it rather than past it. A cap nothing can reach would make the
    // assertions above true for the wrong reason.
    let mut verbose = real.clone();
    verbose.residuals = vec!["P".repeat(8 * MAX_TEXT)];
    let report = model.observe("verbose-record", &verbose);
    assert_findings_bounded(&report, "verbose-record");
    assert!(
        report
            .divergences
            .iter()
            .any(|d| d.kind.wire_name() == "recordTooLarge"),
        "kilobytes of residual text are past the record-size bound, and the record says so"
    );
    assert!(report.unchecked.contains(&"residual-has-a-probe"));
    assert_eq!(report.evaluated, vec!["record-size"]);
    // Detail truncation still applies to an interpretable, in-cap record.
    verbose.residuals = vec!["P".repeat(model.policy().max_text())];
    let report = model.observe("long-residual", &verbose);
    let orphan = report
        .divergences
        .iter()
        .find(|d| d.kind.wire_name() == "residualWithoutProbe")
        .expect("an in-cap orphan residual is reported");
    assert_eq!(
        orphan.detail.chars().count(),
        MAX_RESIDUAL_TEXT,
        "the cap is exact, truncation marker included"
    );
    assert!(
        orphan.detail.ends_with('…'),
        "a truncated detail must say that it was truncated"
    );

    // `maxText: 0` is not a length. The applied bound is floored at 1, and the
    // finding names the bound that was applied — a bound of one character is a
    // real statement about this model (it reads no text) rather than a number
    // measured against.
    let floored = ShadowModel::new(ShadowPolicy {
        max_text: 0,
        ..ShadowPolicy::default()
    });
    assert_eq!(
        floored.policy().max_text,
        0,
        "the policy is what was asked for"
    );
    assert_eq!(
        floored.policy().max_text(),
        1,
        "the applied bound is floored"
    );
    let report = floored.observe("zero-text-budget", &real);
    assert!(
        report
            .divergences
            .iter()
            .any(|d| d.kind.wire_name() == "recordTooLarge"),
        "a one-character bound does not admit a labelled run"
    );
    assert!(
        report
            .divergences
            .iter()
            .filter(|d| d.kind.wire_name() == "recordTooLarge")
            .all(|d| d.detail.contains("text<=1")),
        "the finding must name the bound it applied: {:?}",
        report.divergences
    );
    assert_findings_bounded(&report, "zero-text-budget");

    // A token outside the vocabulary: a refusal, and the relations that needed
    // it are reported unchecked — never as agreement.
    let mut alien = real;
    alien.probes[0].verdict = "probablyFine".to_string();
    let report = model.observe("alien-token", &alien);
    assert_eq!(report.signal, Signal::Underdetermined);
    assert!(report
        .divergences
        .iter()
        .any(|d| { d.kind.wire_name() == "unknownToken" && d.class == DivergenceClass::Refusal }));
    assert!(report.unchecked.contains(&"aggregate-supported-by-probes"));
    assert!(!report.evaluated.contains(&"aggregate-supported-by-probes"));
    assert_findings_bounded(&report, "alien-token");

    // The policy is what makes a namespace's expectations checkable: an
    // unmatched probe budget is a divergence, not a comparison.
    let strict = ShadowModel::new(ShadowPolicy {
        expected_probe_count: Some(3),
        ..ShadowPolicy::default()
    });
    assert_eq!(strict.policy().expected_probe_count, Some(3));
    let real = observe_all(&m).remove(0).record;
    let report = strict.observe("wrong-budget", &real);
    assert!(report
        .divergences
        .iter()
        .any(|d| d.kind.wire_name() == "probeCountDiffersFromPolicy"));
    assert_findings_bounded(&report, "wrong-budget");
}

/// The aggregate relations are two-way: a class must have the probe evidence it
/// names, and it must not carry the evidence that names a *different* class.
///
/// Each case below is a record a real run produced with one field changed, so
/// the contradiction is isolated to the relation under test. A one-way check
/// accepts all three: the hunt derives its verdict in priority order, so these
/// are precisely the records where a shadow model is the only reviewer left.
#[test]
fn the_aggregate_relations_are_two_way() {
    let m = manifest();
    let observed = observe_all(&m);
    let model = ShadowModel::new(m.shadow_policy.clone());
    let recorded = |class: &str| -> RecordedEval {
        observed
            .iter()
            .find(|o| o.record.verdict == class)
            .map(|o| o.record.clone())
            .unwrap_or_else(|| panic!("this namespace records a `{class}` run"))
    };
    // The positive control for every case: the unmutated record is consistent,
    // so a divergence below is the mutation and nothing else.
    for class in [
        "spendableByAnyone",
        "movableByAnyone",
        "requiresProof",
        "notUnderProbes",
    ] {
        let report = model.observe(class, &recorded(class));
        assert_eq!(
            report.signal,
            Signal::Consistent,
            "{class}: a recorded run is self-consistent: {:?}",
            report.divergences
        );
    }
    // One contradicting field, one finding: a record failing two rules at once
    // would not isolate the relation under test.
    let contradiction = |subject: &str, record: &RecordedEval| -> String {
        let report = model.observe(subject, record);
        assert_eq!(report.signal, Signal::Divergent, "{subject}");
        assert_findings_bounded(&report, subject);
        let found: Vec<(&str, &str)> = report
            .divergences
            .iter()
            .map(|d| (d.rule, d.detail.as_str()))
            .collect();
        assert_eq!(
            found.len(),
            1,
            "{subject}: expected exactly one finding, got {found:?}"
        );
        assert_eq!(found[0].0, "aggregate-supported-by-probes", "{subject}");
        found[0].1.to_string()
    };

    // `movableByAnyone` names the preserve shape *only*: a record that also
    // reports a passing attacker sample is describing `spendableByAnyone` and
    // would be derived as one.
    let mut moved = recorded("movableByAnyone");
    let passing = moved
        .probes
        .iter()
        .position(|p| p.verdict == "pass" && p.output == "preserve")
        .expect("the class was recorded from a preserve sample that passed");
    moved.probes[passing].output = "attacker".to_string();
    let detail = contradiction("movable-with-attacker-pass", &moved);
    assert!(
        detail.contains("attackerPass=true") && detail.contains("preservePass=true"),
        "the finding must name the pass that contradicts the class: {detail}"
    );

    // `requiresProof` says nothing passed. A pass of either shape contradicts
    // it, whatever the residuals say.
    let mut proven = recorded("requiresProof");
    let other = proven
        .probes
        .iter()
        .position(|p| p.verdict != "needsProof")
        .expect("the run has a probe that is not the residual's source");
    proven.probes[other].verdict = "pass".to_string();
    proven.probes[other].output = "attacker".to_string();
    let detail = contradiction("requires-proof-with-a-pass", &proven);
    assert!(
        detail.contains("anyPass=true") && detail.contains("`requiresProof`"),
        "the finding must name the class and the pass: {detail}"
    );

    // `notUnderProbes` is the no-pass class, in either shape.
    let mut untouched = recorded("notUnderProbes");
    let probe = untouched
        .probes
        .iter_mut()
        .find(|p| p.verdict != "pass")
        .expect("the no-pass class has a probe to turn into one");
    probe.verdict = "pass".to_string();
    let detail = contradiction("not-under-probes-with-a-pass", &untouched);
    assert!(
        detail.contains("anyPass=true") && detail.contains("`notUnderProbes`"),
        "the finding must name the class and the pass: {detail}"
    );
}

#[test]
fn recorded_expectations_reproduce_and_are_reported_as_self_consistency() {
    let m = manifest();
    let observed = observe_all(&m);
    let exposure = scan_exposure(&m);
    let c = credit(&m.items, &observed, &exposure, &m).expect("this namespace's rows are honest");

    for o in &observed {
        assert!(
            o.reproduced,
            "{}: recorded `{}`, observed `{}`. If the instrument changed, re-record \
             deliberately; never edit the fixture to fit the run",
            o.id, o.expected, o.verdict
        );
        println!(
            "holdout {:<28} {:<15} observed={:<18} selfSynthetic={}",
            o.id, o.role, o.verdict, o.synthetic
        );
    }
    assert_eq!(c.reproduced, c.items);
    assert_eq!(
        c.separating_pairs, c.pairs,
        "every pair must separate on the recorded verdicts"
    );
    println!(
        "\nHOLDOUT self-consistency (NOT discovery credit): {}/{} recorded expectations \
         reproduced; {}/{} pairs separate\nHOLDOUT_CREDIT {}\nHOLDOUT_EXPOSURE {}\nHOLDOUT_NOTE {}",
        c.reproduced,
        c.items,
        c.separating_pairs,
        c.pairs,
        json!({ "credit": c.report() }),
        exposure.report(),
        json!({ "note": m.note }),
    );
}

/// The exposure matcher's positive control. A scan that finds nothing because
/// it can match nothing produces exactly the same numbers as a clean corpus, so
/// the matcher is exercised here against overlaps that are known to exist and
/// against a file known to share nothing. This is also the regression test for
/// clause-level scanning: with whole-line matching, `reused-conjunct` below
/// shares no line with the fixture and would read as clean.
#[test]
fn the_clause_scan_finds_a_shared_conjunct_and_nothing_else() {
    const MIN_CLAUSE: usize = 32;
    const FIXTURE: &str = "\
{ val next = OUTPUTS(0)
  sigmaProp(next.propositionBytes == SELF.propositionBytes && next.value >= SELF.value) }
";
    let corpus = |path: &str, text: &str| CorpusFile {
        path: path.to_string(),
        text: normalise(text),
    };
    let reused_conjunct =
        "sigmaProp(next.propositionBytes == SELF.propositionBytes && next.value >= 1) }";
    assert!(
        !normalise(reused_conjunct).contains(&normalise(FIXTURE)),
        "the control is only meaningful if the corpus file is not a copy of the fixture"
    );

    // A corpus file that reuses one conjunct of the fixture, and no whole line.
    let mut exposure = Exposure::default();
    exposure.measure(
        "row",
        FIXTURE,
        MIN_CLAUSE,
        &[corpus(
            "examples/contracts/recipes/other.es",
            reused_conjunct,
        )],
    );
    assert!(exposure.overlaps("row"), "a shared conjunct is an overlap");
    assert!(
        !exposure.verbatim.contains_key("row"),
        "a shared conjunct is not a copy of the fixture"
    );
    assert_eq!(
        exposure.paths("row"),
        vec!["examples/contracts/recipes/other.es"]
    );

    // A reformatted copy of the whole fixture is verbatim, not merely a clause.
    let mut exposure = Exposure::default();
    exposure.measure(
        "row",
        FIXTURE,
        MIN_CLAUSE,
        &[corpus(
            "examples/contracts/recipes/copy.es",
            "{\n  val next = OUTPUTS(0)\n  sigmaProp(next.propositionBytes == SELF.propositionBytes\n            && next.value >= SELF.value)\n}\n",
        )],
    );
    assert_eq!(
        exposure.verbatim.get("row").map(Vec::as_slice),
        Some(["examples/contracts/recipes/copy.es".to_string()].as_slice())
    );

    // A file whose only shared text is a fragment below the declared clause
    // length is not an overlap: shorter than that, the fragment is a token any
    // two ErgoScript files have in common.
    let mut exposure = Exposure::default();
    exposure.measure(
        "row",
        FIXTURE,
        MIN_CLAUSE,
        &[corpus(
            "examples/contracts/recipes/short.es",
            "// next.propositionBytes == SELF\n{ sigmaProp(true) }",
        )],
    );
    assert!(
        !exposure.overlaps("row"),
        "a fragment below the declared clause length is not identifying: {:?}",
        exposure.paths("row")
    );

    // A file that shares nothing at all.
    let mut exposure = Exposure::default();
    exposure.measure(
        "row",
        FIXTURE,
        MIN_CLAUSE,
        &[corpus(
            "examples/contracts/recipes/unrelated.es",
            "sigmaProp(HEIGHT > 1500000 && SELF.value > 0) }",
        )],
    );
    assert!(!exposure.overlaps("row"));
    assert!(exposure.verbatim.is_empty() && exposure.clauses.is_empty());

    // The real namespace, through the same matcher: at least one fixture is
    // measured against the corpus without being found, which is what makes the
    // zero denominators above a measurement rather than a matcher that never
    // fires. The overlap this finds is the successor-binding idiom, reused by
    // other contracts in the visible corpus.
    let m = manifest();
    let measured = scan_exposure(&m);
    assert!(
        measured.files > 0,
        "the measured scan read nothing, so the control says nothing about it"
    );
    for item in &m.items {
        let source = std::fs::read_to_string(root().join(&item.path)).expect("item source");
        let clauses = clauses_of(&source, m.exposure_scan.min_clause_chars);
        assert!(
            !clauses.is_empty(),
            "{}: the clause scan produced nothing to look for, so its verdict is vacuous",
            item.id
        );
    }
}

/// The manifest is corpus-shaped: it sits inside a declared scan root, carries a
/// declared extension, and holds the fixtures' own diffs and prose. Excluding it
/// is therefore a decision rather than a no-op, and the harness makes that
/// decision so a manifest cannot launder a row by leaving its own path out.
#[test]
fn the_manifest_is_excluded_from_its_own_exposure_scan() {
    let m = manifest();
    let manifest = root().join(MANIFEST_PATH);
    // Measured against a manifest that does *not* name itself, so the harness's
    // own exclusion is what is under test rather than the manifest's self
    // declaration.
    let mut unnamed = m.clone();
    unnamed
        .exposure_scan
        .excluded
        .retain(|p| p != MANIFEST_PATH);
    let declared_only = scan_candidates(&unnamed, &[]);
    assert!(
        declared_only.contains(&manifest),
        "the manifest must be a candidate unless something excludes it, or excluding it is \
         untested and the scan may read the description of the row it judges"
    );
    let excluded = scan_candidates(&m, &[MANIFEST_PATH]);
    assert!(
        !excluded.contains(&manifest),
        "the harness excludes its own manifest whatever the manifest declares"
    );
    assert_eq!(
        declared_only.len(),
        excluded.len() + 1,
        "excluding the manifest must remove the manifest and nothing else"
    );
    for item in &m.items {
        assert!(
            !excluded.contains(&root().join(&item.path)),
            "{}: the namespace must not measure itself",
            item.id
        );
    }
    // What the live manifest declares about itself is belt and braces: the
    // exclusion above does not depend on it.
    let live = scan_candidates(&m, &[]);
    assert!(
        !live.contains(&manifest),
        "{} declares itself corpus material; a row must never be measured against the \
         document that records it",
        MANIFEST_PATH
    );
}

#[test]
fn exposure_is_measured_and_the_discovery_denominator_is_zero() {
    let m = manifest();
    let observed = observe_all(&m);
    let exposure = scan_exposure(&m);
    let c = credit(&m.items, &observed, &exposure, &m).expect("this namespace's rows are honest");

    // The scan is a real measurement: it read the declared roots.
    assert!(
        exposure.files > 0 && exposure.bytes > 0,
        "the scan read nothing, so 'no overlap' would mean nothing"
    );
    for (id, paths) in exposure.verbatim.iter().chain(exposure.clauses.iter()) {
        assert!(m.items.iter().any(|i| &i.id == id));
        assert!(!paths.is_empty());
        for path in paths {
            assert!(
                !path.starts_with("examples/mutants/holdout"),
                "the namespace must not measure itself: {path}"
            );
        }
    }
    // The scan is a measurement of a *clause*, and the matcher is checked
    // against a known overlap above; here the derived numbers are checked
    // against the report they are printed in, so the summary cannot drift from
    // what the walk found.
    assert_eq!(
        exposure.report()["verbatimOverlaps"],
        json!(exposure.verbatim)
    );
    assert_eq!(exposure.report()["clauseOverlaps"], json!(exposure.clauses));
    assert_eq!(exposure.report()["filesScanned"], json!(exposure.files),);
    assert!(
        !exposure.truncated,
        "the scan hit its cap, so a clean result would not be a statement about the corpus"
    );
    println!(
        "HOLDOUT exposure: {} files / {} bytes scanned; verbatim overlaps {}; clause overlaps {}",
        exposure.files,
        exposure.bytes,
        exposure.verbatim.len(),
        exposure.clauses.len()
    );
    for (id, paths) in exposure.verbatim.iter().chain(exposure.clauses.iter()) {
        println!("  {id}: {paths:?}");
    }

    // Every row in this namespace was authored from the instrument's output.
    assert_eq!(c.declared_instrument_visible, c.items);
    assert_eq!(c.declared_independent, 0);
    assert_eq!(
        c.eligible, 0,
        "an author-visible row is never discovery-eligible"
    );
    assert_eq!(
        c.detection_rate, None,
        "a zero denominator must produce no rate at all"
    );
    assert!(
        c.rate_reason.contains("instrument-visible"),
        "the reason must name the exposure: {}",
        c.rate_reason
    );
    let report = c.report();
    assert_eq!(report["discoveryCredit"]["eligibleDenominator"], 0);
    assert_eq!(report["discoveryCredit"]["detectionRate"], Value::Null);
    assert_eq!(report["selfConsistency"]["reproduced"], c.items);
    assert!(report["selfConsistency"]["note"]
        .as_str()
        .expect("a note")
        .contains("NOT discovery credit"));
    assert_eq!(report["discoveryCredit"]["eligibleDenominator"], 0);
}

/// The zero above is a measurement, not a stub: the credit path is exercised
/// with a denominator it is allowed to have, and a row that lies about its
/// independence is refused.
#[test]
fn credit_follows_the_denominator_and_refuses_a_false_independence_claim() {
    let m = manifest();
    let observed = observe_all(&m);
    let exposure = scan_exposure(&m);
    let base =
        credit(&m.items, &observed, &exposure, &m).expect("this namespace's rows are honest");
    assert_eq!(base.eligible, 0);
    assert_eq!(base.detection_rate, None);

    // One genuinely independent row, no measured overlap: a rate appears, and
    // its denominator is that row alone — not the whole namespace.
    let mut clean = m.items.clone();
    let target = clean
        .iter_mut()
        .find(|i| !exposure.overlaps(&i.id))
        .expect(NO_CLEAN_ROW);
    target.authoring = "independent".to_string();
    let with_credit = credit(&clean, &observed, &exposure, &m).expect("one honest new row");
    assert_eq!(with_credit.eligible, 1);
    assert_eq!(with_credit.declared_instrument_visible, clean.len() - 1);
    let rate = with_credit
        .detection_rate
        .expect("a non-empty denominator yields a rate");
    assert!((0.0..=1.0).contains(&rate), "a rate is a fraction: {rate}");
    assert!(with_credit.rate_reason.contains("1 eligible item"));
    assert_eq!(
        with_credit.items,
        clean.len(),
        "self-consistency still covers every row, eligible or not"
    );
    // Self-consistency is not discovery credit: the eligible row reproduced too,
    // and the two numbers stay separate.
    assert_eq!(with_credit.reproduced, base.reproduced);
    assert!(
        !with_credit.rate_reason.contains("instrument-visible"),
        "with an eligible row the reason is the derived denominator, not the exposure story: {}",
        with_credit.rate_reason
    );

    // An `independent` row the measurement contradicts is refused, with the
    // paths that contradict it.
    let mut lying = m.items.clone();
    let liar_id = {
        let liar = lying
            .iter_mut()
            .find(|i| !exposure.overlaps(&i.id))
            .expect(NO_CLEAN_ROW);
        liar.authoring = "independent".to_string();
        liar.id.clone()
    };
    let contradicted = Exposure {
        verbatim: BTreeMap::from([(
            liar_id.clone(),
            vec!["examples/contracts/recipes/time-lock.es".to_string()],
        )]),
        ..Exposure::default()
    };
    let refusal = credit(&lying, &observed, &contradicted, &m)
        .expect_err("a row that claims independence while the scan finds it must fail");
    assert!(refusal.contains(liar_id.as_str()), "{refusal}");
    assert!(
        refusal.contains("time-lock.es"),
        "the refusal must cite what contradicts it: {refusal}"
    );

    // An unknown `authoring` value is refused too: the vocabulary is a policy
    // decision, not free text.
    let mut alien = m.items.clone();
    alien[0].authoring = "independent-ish".to_string();
    let refusal = credit(&alien, &observed, &exposure, &m)
        .expect_err("an authoring value outside the declared vocabulary must fail");
    assert!(refusal.contains("independent-ish"), "{refusal}");
}

/// A stated instrument limit, measured rather than asserted: the H02
/// counterexample retains `OUTPUTS.size == 1`, and every probe builds exactly
/// one output, so the clause is inert under these probes. The holdout says so;
/// this test proves it, so the claim cannot rot unnoticed.
#[test]
fn the_output_count_clause_is_invisible_to_this_instrument() {
    let m = manifest();
    let pair = m
        .pairs
        .iter()
        .find(|p| {
            p.instrument_limit
                .as_deref()
                .is_some_and(|limit| limit.contains("OUTPUTS.size"))
        })
        .expect("a pair that states the output-count limit");
    let source = std::fs::read_to_string(root().join(&pair.counterexample_path))
        .expect("counterexample source");
    assert!(
        source.contains("OUTPUTS.size == 1"),
        "{}: the pair no longer states the limit it documents",
        pair.id
    );
    let without = source.replacen("OUTPUTS.size == 1 && ", "", 1);
    assert_ne!(
        without, source,
        "{}: the documented clause is gone",
        pair.id
    );

    let run = |src: &str| {
        let compiled = compile::compile_source(src, m.caps.tree_version, network(&m.caps.network))
            .expect("both spellings compile");
        let report = hunt(
            &compiled.tree_bytes,
            &HuntOptions {
                height: Some(m.caps.height),
                network: Some(network(&m.caps.network)),
                ..HuntOptions::default()
            },
        )
        .expect("hunt runs");
        let json = serde_json::to_value(&report).expect("the hunt serialises");
        (
            json["verdict"].as_str().expect("verdict token").to_string(),
            json["probes"].as_array().map(Vec::len).expect("probes"),
            json["caps"].clone(),
        )
    };
    let (with_clause, probes_with, budget_with) = run(&source);
    let (without_clause, probes_without, budget_without) = run(&without);
    assert_eq!(probes_with, probes_without, "the same probe budget ran");
    assert_eq!(budget_with, budget_without, "the same cap policy ran");
    assert_eq!(probes_with, m.caps.max_probes);
    assert_eq!(
        with_clause, without_clause,
        "{}: the stated instrument limit no longer holds. A multi-output probe would change \
         this pair's evidence, and the manifest must then be updated deliberately",
        pair.id
    );
}

#[test]
fn an_oversized_record_leaves_relations_and_policy_unchecked() {
    let policy: ShadowPolicy = serde_json::from_value(serde_json::json!({
        "expectedProbeCount": 2, "requirePairedOutputShapes": true, "requireProbeLabels": true
    }))
    .unwrap();
    let record: RecordedEval = serde_json::from_value(serde_json::json!({
        "method": "hunt", "verdict": "spendableByAnyone", "probes": [],
        "observation": "x".repeat(100_000), "nodeValidated": true
    }))
    .unwrap();
    let report = ShadowModel::new(policy).observe("oversized", &record);
    assert_eq!(report.signal, Signal::Divergent);
    assert_eq!(report.evaluated, vec!["record-size"]);
    assert_eq!(report.divergences.len(), 1);
    for rule in [
        "aggregate-supported-by-probes",
        "token-vocabulary",
        "no-node-validation",
        "policy.probe-count",
        "policy.shape-pairing",
        "policy.probe-labels",
    ] {
        assert!(report.unchecked.contains(&rule), "{report:?}");
    }
}
