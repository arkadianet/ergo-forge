# Audit discovery workbench

This branch adds bounded instruments for finding and reproducing contract
counterexamples on top of the pinned Forge engine. It does not certify safety.

## What is new

- **Audit coverage** — `ergo_sandbox::audit::coverage` projects the embedded
  review catalogue and the audit registry over a completed static audit. It
  reports which registered instruments ran, how many observations each
  recorded, and whether recovery was partial. Availability is read from the
  audit registry (`ergo_sandbox::audit::LINTS`), never from the catalogue: a
  name a document carries but no registered lint runs is reported as
  unregistered, not as an instrument that ran. `ran` means the registered lint
  executed over the recovered tree; it does not mean the lint's own analysis
  bound examined every construct, and a complete recovery is not a clean
  contract. It emits no score, percentage, ranking or safety verdict.
- **Property templates** — `ergo_sandbox::properties::template` expands bounded
  `property-template:v1` documents into the existing `author-property:v1`
  schema. The expanded text must pass `Declaration::parse`; a template never
  creates authority or a declaration digest on its own.
- **Bounded adversary search** — `ergo_sandbox::adversary::search` samples
  seeded multi-step Play traces from the existing adversarial operation
  algebra, carries outputs forward, and delta-debugs the first verdict change.
  Reports carry caps, truncation, rejections, provenance and a replay
  fingerprint. Carried boxes are recreated with the same contract shape and
  evaluated once before mutation, spending a probe, so a depth-two step is
  compared with the unmutated draft it actually changes. A carried draft whose
  balances or construction fail is rejected; inputs that lost their proofs in
  the carry still serve as a parent. Reports are always synthetic and never
  node-validated.
- **Flow review obligations** — `ergo_sandbox::audit::flow` performs bounded,
  fail-closed source-to-sink analysis over the lifted tree. Unmodelled values
  taint rather than sanitise. `flow_paths` renders the result as review
  priority, never as an exploit claim.
- **Holdout and shadow model** — `examples/mutants/holdout.json` is a versioned
  (`holdout/v1/`) cohort, currently instrument-visible, whose
  discovery-eligible denominator is derived and zero; no rate is reported. Rows
  declare an `authoring` value and are *measured* for exposure, never granted it
  per row. `ergo_sandbox::shadow_model` checks bookkeeping consistency only: it
  re-derives each aggregate class's relation to its probe records in both
  directions, refuses to duplicate consensus, and cannot produce an acceptance
  or property-violation verdict.

## CLI

```text
ergo-es checklist <source> --json
ergo-es property-template <template.json> --json
ergo-es adversary <request.json> --json
ergo-es shadow-check <record.json> --json
```

The adversary request is a Play draft plus an `options` object:

```json
{
  "height": 1000,
  "boxes": [],
  "tx": {"inputs": [], "dataInputs": [], "outputs": []},
  "options": {
    "seed": "review-2026-09-26",
    "maxDepth": 2,
    "maxProbes": 32,
    "maxOpsPerStep": 3,
    "maxUnspent": 24
  }
}
```

`NoFlipUnderProbes` means only that the recorded seed, draft, operation
families and caps found no verdict change. It is not a safety result.

## HTTP

- `POST /api/v1/checklist` now includes `coverage` and `flow` beside the
  existing provenance-labelled rows.
- `POST /api/v2/adversary` accepts the same bounded request as the CLI and
  runs behind the shared engine budget.

## Evidence boundary

The pinned node validator remains the only source of node acceptance. Static
observations, Play/attack/adversary reductions, preflight checks, holdout
self-consistency and the shadow model all report `nodeValidated: false` and
must not be described as accepted execution, confirmed violations or proof of
exploitability.

Every search result records the seed, caps, truncation, rejections and report
fingerprint. Every holdout row declares its `authoring` and is measured for
exposure by a clause-level scan of the declared roots; the discovery denominator
is derived from those two facts, instrument-visible rows earn no discovery
credit, and a zero denominator yields no rate at all.

## Known limits

The flow pass is syntax-directed and bounded. The adversary search is bounded
enumeration driven by a seed, not exhaustive search. A generated decoy is
synthetic and is shaped from the target's current tree; a later decoy in the
same sequence may therefore be generic rather than shaped from the original
script. Property templates only shape author declarations; they do not verify
that a declaration expresses the author's intent.

The shadow model is a bookkeeping consistency model, not a second reducer. Its
relations are two-way — an aggregate class must have the probe evidence it names
*and* must not carry the evidence that names a different class — so it catches a
record that contradicts itself on the same axis, and nothing else: it reads
labels, counts and tokens, and never parses a record's prose. Every count and
text field it reads is capped, a record past a cap is a divergence rather than a
slower check, and each finding's detail is at most `MAX_RESIDUAL_TEXT`
characters. Only the recorded input and the policy deserialise: the report, the
refusal and the finding types are write-only, so a document read back as a
report — including one claiming `nodeValidated: true` — cannot enter the model's
vocabulary.

The holdout exposure scan is a *text* measurement, not a provenance claim. It
compares normalised clauses, split on newlines, `&&`, `||` and `;`, and only
fragments at or above the manifest's `minClauseChars` count; a shared clause is
found, a shared idea that is worded differently is not. Its roots, extensions
and caps are declared in the manifest, and both the namespace and the manifest
itself are excluded from it, so a clean result means "no overlap in what was
read" rather than "independent of the instrument".
