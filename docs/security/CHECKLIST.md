# Contract review checklist

`POST /api/v1/checklist` and `ergo-es checklist` list every S00 vector in catalogue
order. There is no score. A row starts `unchecked`; a lint miss is not a safety
answer. Static observations set review priority and never assert a vulnerability.
The top-level claim is always `static-analysis`, `nodeValidated: false`.

The library entry is `checklist::checklist(bytes, network, &Artifacts)`, with
`checklist::resolve_input` for offline hex, address or source resolution. The HTTP
request takes exactly one of `input` (hex or address, resolved like inspect) and
`source` (compiled on the engine thread at tree version 3), plus an optional
`network` (`mainnet` by default or `testnet`). The route uses the shared engine
permit, large stack, rate limiter, body limit and inspect input-size limit.

```sh
ergo-es checklist examples/contracts/vectors/zero-threshold/vulnerable.es --json
ergo-es checklist contract.es --scenario scenario.json --evidence bundle.json
```

Optional artifacts are envelopes with explicit caller associations:

```json
{
  "source": "sigmaProp(HEIGHT >= 100)",
  "scenario": {
    "vectorIds": ["permissionless-height-release"],
    "scenario": {"height": 200}
  }
}
```

The HTTP `scenario` field and CLI `--scenario` file contain
`{"vectorIds": ["..."], "scenario": <ordinary scenario JSON>}`. Omitted scenario
source/tree uses the checklist contract; an explicit different contract is an
error. Evaluation runs afresh. `scenario` provenance is synthetic and remains
`nodeValidated: false`, even when a supplied proof verifies.

HTTP also accepts `preflight` with
`{"vectorIds": ["..."], "request": <validate-tx request JSON>}`. It runs txcheck
offline over supplied boxes and must spend the checklist contract. It performs
selected script and balance checks without signatures; its provenance is
`preflight`, `nodeValidated: false`. No explorer lookup fills absent boxes.

The HTTP `evidence` field and CLI `--evidence` file contain
`{"vectorIds": ["..."], "bundle": <P05 ReplayBundle>}`. A fresh offline replay must
accept the transaction and evaluate its declared property before the artifact
can label a row `node-validated`. The exact checklist script must occur on a
spending input bound by that property, and any supplied case target must match.
Stored acceptance flags are not evidence. Rejected, incomplete or unsupported
replays are retained as unchecked artifacts. Node acceptance says nothing about
historical state or inclusion; the returned replay scope and premise provenance
remain visible. Neither acceptance nor a declared extraction violation proves
the associated vector's mechanism.

Only vectors naming the relevant instrument accept associations (preflight uses
the catalogue's scenario/drain instruments). Unknown, duplicate or incompatible
vector IDs are errors. Absent `vectorIds` means no row attribution: the artifact
still runs and its result is visible. The CLI also accepts raw scenario and raw
P05 bundle files in this unassociated form. This avoids guessing a review question
from a generic experiment.

Every row carries `id`, `class`, `title`, `answer`, `provenance`, `findings` and
`artifactFingerprints`. Static findings come directly from `audit::audit` and
only from lints the vector names; each includes its unchanged text, lint ID,
`anchor.nodeId`, nullable `anchor.irId`, snippet and `static` provenance. No other
static recogniser is currently named in the catalogue. Recovery completeness is
reported separately and does not measure review completeness.

When several kinds of evidence are supplied, the displayed row answer uses
node-validated, then preflight, then scenario, then static observations. All
static findings and all associated artifact fingerprints remain on the row;
the separate artifact list preserves every run's answer and provenance. Unchecked
artifacts never replace an existing observation. This order chooses an answer's
provenance; it does not rank contracts or establish safety.

Read's `POST /api/v1/inspect` response also includes `negativeSpace`. This is a
projection of that response's existing audit, so plain words, findings and
negative space refer to the same recovery without a second analysis pass. Each
line preserves the instrument's text, lint, anchor and snippet and is visibly
labelled `static` beneath the plain words. It covers the existing observations
about positional reserves, output tails, successor fields, register presence
and trust, writable sigma alternatives and unauthenticated code. No additional
AST analysis or absence claims are introduced. Silence is not a claim that all
boxes, outputs or registers are constrained.
