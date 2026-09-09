# P01 evidence cases: retained premises, no acceptance claim

`evidence::EvidenceCase` is a versioned JSON input record. Every case writes
`formatVersion: 1`, `deploymentIdentity: "unknown"`, and `nodeValidated: false`.
Import rejects unsupported versions, acceptance claims, unknown schema fields,
and missing required premises. It does not import a cached verdict. This is
separate from the legacy triage record's `formatVersion: 2`.

`CasePremises` retains the pinned engine revision, exact target hex, source
identity and optional text, typed constant bindings, original box documents,
execution context, and named assumptions. Each premise explicitly records
`status: "missing"` with a reason, or `status: "present"` with a value and origin.
An explicit empty array/object and a missing premise have different identities.
Roles, budgets, protocol NFTs, objectives, default messages and hypothetical
context values belong in the retained premises; P01 does not interpret them.

Origins describe how a value entered the record. Inferred constants and type-only
overrides remain `hypothetical`; value overrides are `caller-supplied`.
`source-recorded` means an observation was recorded, not that a box was unspent
or its state was independently verified. The only `independently-checked` output
P01 produces is a recomputed SHA-256 of attached source text, explicitly labelled
`sha256-of-attached-source-only`. Caller-imported independent state/binding claims
are rejected. Hash consistency does not authenticate a source or establish that
attached source and target bytes correspond to a deployment.

## Save and inspect through the library

```rust
use ergo_sandbox::{evidence::EvidenceCase, ingest::{ingest_source, IngestOptions}};

let result = ingest_source("sigmaProp(HEIGHT > limit)", &IngestOptions::default());
let artifact = result.artifact.unwrap();
let saved = serde_json::to_string_pretty(artifact.evidence_case().unwrap()).unwrap();
let imported: EvidenceCase = serde_json::from_str(&saved).unwrap();
let analysis = imported.analyze().unwrap();
let inspection = analysis.result_for(&imported, &[]).unwrap();
assert_eq!(inspection.findings.len(), artifact.analyze().unwrap()
    .result_for(&imported, &[]).unwrap().findings.len());
```

`analyze()` uses the existing parser, lifter, decompiler and audit rules and wraps
the result with its complete case. `identity::match_cases(left, right)` retains
both cases; even byte equality leaves deployment identity unknown. Both operations
reject absent target bytes or a different engine revision. Existing byte-only
functions still return their legacy static results; they cannot construct the new
analysis wrapper. Editing an artifact's public legacy bytes invalidates its
provenance-bearing adapter, which also re-lifts the bound bytes rather than trusting
mutable legacy IR. Ingestion reports include their case on failure as well as
success; directory JSON output carries these cases without a new CLI operation.

`Analysis::result_for` requires the original case and every comparison dependency.
Its key covers all serialized premises and origins, plus the analysis version.
Changing any premise invalidates reuse, including a source locator, missing-value
reason, context default, or policy assumption. Object key order is immaterial;
array order is retained. Cases can be imported and analyzed afresh; serialized
analysis results are inspection records and have no trusted deserializer. These
JSON fingerprints are cache identities, never Ergo box or transaction IDs.

## Preserve observations before legacy defaults

`map::source::record_box` captures original JSON with a source locator, optional
revision, and document digest. Call it **before** deserializing a legacy `ChainBox`.
Omitted, null and explicitly empty registers remain distinct raw records;
`registers()` reports omitted/null registers as missing. A defaulted `ChainBox`
cannot recover that distinction. Map traversal retains its legacy behavior; this
adapter neither proves historical membership nor implements canonical box bytes.

`ingest::recorded_inventory` reads the final `after.contracts` rows and `files`
identities in the archived ingestion report. It retains all 28 rows, including
the two failures, with recorded binding values, mechanisms, source lines and
origins. The binding set is partial for failed compilation. Source text and target
bytes were not retained in that archive, so they stay missing and cannot be
analyzed by guessing or recompiling them. The original measurement row and an
identity of the inventory are included in each case. No recorded corpus data is
rewritten and compilation coverage is not deployment coverage.

P01 stops at saving and inspecting premises. There is no wire codec, validator,
transaction replay, browser evidence import, new detector or search axis here.

P02 adds the separate [node wire construction path](evidence-wire.md). The P01
input schema and legacy functions above retain their meaning; wire construction
still cannot claim transaction acceptance.
