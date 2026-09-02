# P3b — the spend hunt ("spendable by anyone?")

> Design record, 2026-09-02. Companion to the P3a lints spec; same audit layer,
> different instrument. Lints read the tree; the hunt *runs* it.

## The question

An auditor's first question about an on-chain contract is not "is the code
tidy" but **"can someone who holds no key spend this box?"** The static lints
cannot answer it — a guard can be present and still be satisfiable by anyone.
The sandbox can: it evaluates the tree on the consensus reducer with a chosen
context and no proof. If the tree reduces to `TrivialProp(true)`, the box is
spendable by whoever built that context.

The hunt is **bounded scenario sampling** over the sandbox, exactly as the plan
scoped it ("semantic equivalence belongs to the audit layer, not decompiler
CI"). It is a hunt, not a proof: a hit is real (the context that produced it is
a transaction anyone can build), a miss says only "not under these probes".

## Probes

A probe is one scenario built from the tree plus the caller's optional
description of the spent box. Every probe supplies **no proof and no context
variables** — that is what "anyone" means. Probes vary the two things an
attacker controls freely:

| Axis | Values | Why |
|---|---|---|
| Height | `base`, `base + 1_000_000`, `1` | Height guards. `base` is the caller's height or a default near the current chain tip; `+1M` finds "after the deadline" unlocks; `1` catches inverted guards. |
| Outputs | **attacker**: one box, same value/tokens as SELF, attacker's tree · **preserve**: one box copying SELF entirely | A tree that passes with the attacker output is *stealable*. One that passes only with SELF preserved is *movable by anyone* — funds stay in the contract, often by design (oracle pools, refresh boxes). |

Inputs are `[SELF]`; data inputs empty; pre-header zero. The attacker tree is
`sigmaProp(true)` compiled by the oracle-pinned compiler (not a hand-written
constant).

The caller may pass a `selfBox` (`ScenarioBox`: value, tokens, registers,
creation height). Without it SELF is synthetic — no registers, value 0 — and
any script reading `SELF.R4` errors out, which is a **false negative** the
report must say out loud. This is why the hunt takes the box description now:
the UI has no explorer access (no outbound calls), but a user can paste one.

Six probes, each a full consensus reduction with the default block-cost
budget. Sub-millisecond each on real trees; the hunt is cheap enough to run on
every inspect.

## Verdict

Per probe: the sandbox `Verdict` (`pass` / `fail` / `error` / `needsProof`)
plus `reducedTo` (the residual sigma proposition, e.g. `ProveDlog(02…)`) and
the runtime error text when there is one.

Aggregate, in priority order:

1. **`spendableByAnyone`** — some *attacker* probe passed. High. The finding
   names the probe (height, output shape).
2. **`movableByAnyone`** — no attacker probe passed, some *preserve* probe did.
   Medium. "Anyone can re-spend this box back into the same contract."
3. **`requiresProof`** — nothing passed; at least one probe reduced to a sigma
   proposition. The distinct residual propositions are reported — this is
   "who can spend": a `ProveDlog` is a key, a `THRESHOLD` is a multisig.
4. **`notUnderProbes`** — every probe failed or errored. Explicitly *not*
   "safe". With a synthetic SELF and any register read, this is the expected
   result and the report says so (`selfSynthetic: true`).

`error` verdicts are kept per probe with their text: "Option.get on None"
under a synthetic SELF is the signal to supply the real box.

## Surfaces

- **Engine:** `ergo_sandbox::hunt::{hunt, HuntOptions, Hunt, Probe, HuntVerdict}`.
  `hunt(tree_bytes, &opts) -> Result<Hunt, SandboxError>`; errors are
  marshalling only (bad tree), never a script outcome. Builds `Scenario`s and
  calls `eval_scenario` — no new evaluator entry, the one-primitive rule holds.
- **CLI:** `ergo-es hunt <tree-hex> [--height N]` and `ergo-es hunt --mainnet [N]`
  for the measured rate over the corpus (the P3a acceptance pattern: the tally
  is the stdout output; hits go to stderr for hand verification).
- **Web:** `POST /api/v1/hunt` `{input, network?, height?, selfBox?}` → the
  `Hunt` as JSON. Same input resolution and large-stack blocking task as
  inspect (the reducer recurses too). Shares the inspect route's concurrency
  bound by sitting under the same layer.
- **UI:** a "Spendability" section under the findings, filled by a second
  request after inspect succeeds. Verdict line, then the probe table.

## Verification

- Unit/integration tests on authored sources: `sigmaProp(true)` →
  `spendableByAnyone`; `sigmaProp(HEIGHT > base+10)` → spendable at `base+1M`
  only; a P2PK tree → `requiresProof` with the `ProveDlog` residual;
  `sigmaProp(OUTPUTS(0).propositionBytes == SELF.propositionBytes)` →
  `movableByAnyone`; `SELF.R4[Int].get > 0` with no box → `notUnderProbes`,
  `selfSynthetic`, error text present; with a box carrying `R4 = 5` → spendable.
- Measured on the 279-tree mainnet corpus and recorded in the plan, with the
  hits hand-checked against their decompiled source.

## Out of scope (recorded so they are not rediscovered)

- Register/context-var fuzzing (random typed values). Needs type information
  from the lifted tree to be non-trivial; a later increment.
- Data-input probes. Real contracts read oracle boxes; without explorer access
  the user would have to paste them. The `ScenarioBox` model already supports
  it — the API can grow `dataInputs` without a redesign.
- "Which key" beyond the residual proposition: mapping `ProveDlog` bytes to a
  P2PK address is presentation, belongs in the UI when it is wanted.
