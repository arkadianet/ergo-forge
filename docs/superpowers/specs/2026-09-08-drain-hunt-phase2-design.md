# Drain hunt phase 2 — output synthesis

> Design record, 2026-09-08. Companion to the phase-1 spec
> (`2026-09-08-drain-hunt-phase1-design.md`, merged in #61; implementation in
> #64). Phase 1 deferred this in one line: *"Free-form output synthesis
> (attacker invents outputs) — phase 2."* This is a **design doc** — no
> implementation. It exists to settle two decisions before any code is
> written, because both are cheaper to get wrong on paper.

## What phase 1 left on the table

Phase 1 varies only what an attacker controls about **composition**: the order
of inputs, the contents of attacker-owned slots, and the recipient of outputs
the caller marked `payee: free`. The output *shape* — how many outputs, of
what value, carrying which tokens, under which scripts, at which index — stays
exactly as the caller declared it.

That was enough to rediscover the USE LP drain, because that attacker reused
the honest builder's shape and only reordered inputs. It was enough by luck of
that bug, not by construction. A whole family of drains is unreachable while
the output shape is fixed:

- value split across **more outputs** than the template has;
- reserves leaving in a **token layout** the template never contained;
- a successor recreated at a **different index**, or not at all, where the
  script only checks "some output" rather than `OUTPUTS(0)`;
- a drain whose payout box **does not exist** in the honest transaction;
- a spend whose authorization lives in the **shape of an output** — the
  concrete case this incident left open.

The concrete case first, because it is funded and unresolved. The USE bank
vault (`e6162e2aff23…`, 292,615.109709675 ERG plus the ~1e15 USE treasury)
authorizes a spend when `OUTPUTS(0).tokens(2)._1` is one of four admin NFTs
(`useFreeMint`, `useArbitrageMint`, `usePayout`, `useUpdateNft` — or the
never-minted `dbf655…` at `OUTPUTS(2)`, a dead branch). Phase 1 structurally
cannot express that shape: its `OUTPUTS(0)` is always the first protected
successor, whose token layout the caller declared. The phase-1 hunt therefore
answers `notUnderProbes` on the vault set — a statement about the probes, not
about the vault. Whether the vault can be drained without a key (the four
authorizer companions sit right there in the map, and three of their scripts
have never been fully decoded) is exactly the question phase 2 exists to
answer mechanically.

## What carries over unchanged

Phase 2 is a new probe generator, not a new hunt. These are fixed:

- **Roles** — caller-supplied, map-fed, `unknown` refused.
- **The oracle** — full `txcheck::check` validation, exact conservation, with
  `pass`-gating on `protected`/`companion` inputs (`needsProof` there is an
  unheld key; the attacker signs only their own boxes).
- **The leak objective** — identity and amount: `protectedIn` from
  `protected`-labelled inputs; script-matched successors contribute their
  *actual* holdings per asset; sanctioned outflow is exactly what the caller
  declared through free payees. No binding is unique box identity by default;
  `propositionBytes ==` pins script bytes only.
- **Verdicts and honesty** — `drainable` / `notUnderProbes` /
  `invalidShape`; a miss names the probe space and is never "safe".
- **The witness bundle** — scenario + roles + mechanically derived
  `TxRequest`.
- **Compatibility** — phase-1 request JSON stays valid. Synthesis is opt-in
  through a `synthesis` block (below); without it the hunt is byte-for-byte
  phase 1, and every existing suite keeps its verdict.

## Decision 1 — the space stays a declared finite family, not a search

Free-form synthesis is unbounded: any output count, any value split, any token
layout, any recipient. An unbounded space cannot be enumerated, and a sampled
unbounded space cannot honestly be called a sweep. **The decision: phase 2
stays exhaustive over a declared finite family.** It does not become a search
(coverage-guided steering remains phase 3). Output construction is a finite,
deterministic, **script-independent** enumeration over these degrees of
freedom, each capped and each recorded in the report:

1. **Synthesized boxes.** Up to `synthesis.maxNewOutputs` (default 2) new
   outputs beyond the template, of two kinds:
   - **Companion re-creations with padded token layouts** — a companion box
     rebuilt verbatim (same script, value, tokens) with filler tokens
     inserted so that a carried NFT lands at an attacker-chosen index
     `i ∈ {0..=3}`. This is the vault move: the whitelisted NFT at
     `OUTPUTS(k).tokens(2)`.
     **The padding has a token source, and saying so is load-bearing.**
     Conservation rejects any output id the inputs do not carry, and a box
     cannot hold one id twice — so reaching index 2 needs *two distinct
     filler ids actually held by inputs*. The fillers are therefore sourced,
     never conjured: the generator pairs a padded re-creation only with the
     decoy combination that places those exact filler ids on attacker input
     slots (one filler id per index, deterministically paired), and a
     caller-declared attacker input that already carries the needed ids
     sources it too. A padded probe whose fillers are not sourced is not
     generated at all — generating it would fail conservation and make the
     flagship verdict vacuous ("conservation blocked us" wearing the costume
     of "the scripts refused"). The report must keep the two apart: every
     synthesized probe's rejection is classified (`conservation` /
     `script` / `missing-key`) and tallied, from `txcheck`'s problems and
     per-input verdicts.
     Whether the rebuilt companion's *own* script still passes is not our
     problem to decide — the oracle judges it, and its time-locks and guards
     are exactly what the probes exercise.
   - **New attacker sinks** — boxes under the attacker tree, receiving value.
2. **Per-successor state.** Phase 1 shrank all successors simultaneously in
   drain mode. Phase 2 makes it independent: each script-matched successor is
   `{verbatim, minimized}` (`DRAIN_KEEP_VALUE` + one token per declared id),
   `2^s` combinations capped at 8 — protocols with multiple protected boxes
   where only one is exploitable become expressible.
3. **Value splitting.** A sink receives either everything (the phase-1
   workhorse) or the first deterministic split (`⌈half⌉`/`⌊half⌋` across two
   sinks). Finite; the split exists because two-box layouts are the common
   laundering shape.
4. **Mint probes.** Conservation allows one minted token id — the first
   input's box id. One synthesized output may mint it, in amounts drawn from
   the **declared request alone**: `{1, the largest amount of any token held
   by a declared input, the largest amount declared on any template
   output}` — never from the target script, because a mint sized to
   "whatever a synthesized check would want" is script-dependent, which is
   the one property the anti-cheat rests on. Minted placeholders forge
   "successor-looking" boxes for protocols that check a token id the caller
   did **not** declare as a protocol NFT — and the map's token evidence (`TokenClass::Singleton` vs `Fungible`, from #63's chain source)
   is what makes the distinction honest: a declared singleton cannot be
   forged by minting, because its id is not the first input's box id and
   conservation blocks the fake.
5. **Output permutation.** Outputs join the input permutation domain
   (capped, lexicographic, deterministically sampled on overflow) —
   synthesized boxes must be able to land at pinned indices like
   `OUTPUTS(0)`. Phase 1 kept output order fixed; phase 2 makes order
   symmetric.

The probe count is the product of the enabled degrees, and for any
realistic set the product exceeds the total-probe cap — **truncation is the
normal case, not the overflow case**. "Exhaustive over the family" therefore
means: exhaustive within the sampled prefix, where the prefix is defined by a
**pinned axis order**, outermost first: (1) synthesized-output shapes (none →
companion re-creations → sinks — the re-creations come before the sinks
because the vault move is the class the incident left open, and axis (1) is
the first axis truncation reaches); (2) output permutation; (3) per-successor
states;
(4) value splits; (5) mint variants; (6) input permutation and decoy
combinations, the phase-1 axes, innermost. Synthesis axes iterate outermost
and input axes innermost: truncation then preserves the new degrees and
spends its budget varying the phase-1 space inside each synthesis shape,
instead of the reverse. Caps: `maxNewOutputs` (2), successor states (8),
output permutations (24 default), total probes (50,000 default) — every cap
a parameter, every cap recorded in the report alongside the pinned order.
With the default caps the sampled prefix covers a bounded slice, so the
flagship vault run **declares larger caps explicitly** (~200,000 probes —
minutes, not hours) and records them. The honesty rule from phase 1 carries:
a miss says "not under these probes", names which synthesis degrees were
enabled, and names the caps and the truncation order.

The anti-cheat carries too, sharpened: the vault acceptance below must be
won by the **generic family** — a hand-fed `OUTPUTS(0)` with the right NFT at
the right index is not a finding, it is a fixture bug.

## Decision 2 — what "sanctioned" means once outputs are invented

Phase 1's leak subtracts the declared free payees' amounts. Once the attacker
invents outputs, the accounting needs three rules, all extensions of the
phase-1 objective rather than changes to it:

1. **Synthesized boxes sanction nothing.** Only the caller's declared free
   payees (at their declared amounts) count as sanctioned outflow. A
   synthesized sink holding protected value is leak, all of it.
2. **A fabricated successor counts as protected only when the NFT rides
   along.** A synthesized output is counted into `protectedOut` only when it
   carries the protected input's protocol NFT — same id, same token index —
   in addition to matching the script bytes. Script equality alone is not
   enough: with synthesis, the generator could otherwise walk into a false
   negative — park the reserves in a script-clone that does not carry the
   NFT, score zero leak, and walk away. Reserves detached from their
   singleton is a real break of exactly the class this project exists to
   catch, so it is reported as its own signal (`nftDetached`): any
   script-matched output that lacks the protocol NFT is named in the report
   whether or not the probe drains. When synthesis is off, the phase-1 rule
   stands unchanged (caller-declared outputs are trusted) and behavior is
   byte-identical.
3. **Minted tokens are neither protected nor sanctioned.** A mint id cannot
   appear in `protectedIn` (no protected input holds it — and if one
   coincidentally does, conservation accounts the movement), and a minted
   amount never reduces the leak. It can only make a probe *valid* that
   would otherwise fail conservation — which is the point: some protocols
   check output token ids against constants, and a minted placeholder is how
   an attacker satisfies them.

One fidelity limit must be recorded rather than hidden: `txcheck` does not
model storage-rent minimums, so a synthesized output with a dust value would
pass the oracle and be rejected by consensus — a false witness. Synthesized
value-carrying boxes therefore floor at [`DRAIN_KEEP_VALUE`] (the recorded
observed floor, already a knob), and the report names the limit.

## The flagship acceptance: the bank vault, answered empirically

The vault set comes from the merged map (`use-lp.json`, which carries the
vault post-drain — the vault was untouched: 292,615 ERG and the USE treasury
are its current state) plus the incident corpus's authorizer boxes. The
phase-2 acceptance is deliberately two-sided, because the honest answer is
not known in advance — three of the four authorizer scripts have never been
fully decoded, and the hunt is precisely the instrument that answers it
without the decode:

1. **The admin route is refused, by construction.** Carrying `useUpdateNft`
   means spending the admin's P2PK box — `needsProof` on a companion input,
   disqualified by the phase-1 gate. The test asserts no witness carries the
   admin NFT. This is deterministic and asserted unconditionally.
2. **The other three routes are empirical.** `usePayout` (5040-block
   cadence, `HEIGHT == SELF.R4 + 5040`), `useFreeMint` and
   `useArbitrageMint` (their own guards and time-locks) are generated,
   validated, and scored like any probe. The hunt's verdict on the vault set
   is run once, recorded in the incident corpus, and the test pins the
   recorded verdict — the same pattern the USE replay used: the corpus is
   both fixture and answer key.

   **Sources for the whitelist shape** — this spec's flagship hangs on it,
   so the provenance is stated: the vault box
   `e6162e2aff23f7c88968cc958541bacfe3ad80d6541befb7231ac3106e966f8b`
   (carried by the merged map fixture `use-lp.json`), whose deployed
   ErgoTree constants were hand-decoded during the incident response — the
   four NFT constants and the dead `dbf655…` branch at `OUTPUTS(2)` are
   visible in the tree's constant block, and the drain analysis in the
   incident report records the decode. **Do not confuse this with**
   `examples/contracts/dexy/bank/bank.es`, the DexyGold fork's bank, which
   checks `INPUTS(mintInIndex).tokens(0)._1` — an input-side, index-0
   check; a different contract. Implementation must land the vault's
   deployed tree as a corpus fixture so the asserted whitelist shape is
   checkable, not taken on trust from this document.
3. **Either outcome is a result.** `drainable` means the vault is a live
   keyless drain of 292,615 ERG + the treasury: **disclosure-first** — the
   operator is the project itself here, but the rule from the phase-1 spec
   stands for any third-party set: private report, corpus entry only
   post-fix or on an agreed window. `notUnderProbes` means the four
   authorizer scripts (whatever they say) close the whitelist in every
   attacker-buildable shape phase 2 can express — the strongest statement
   about the vault anyone has made, and the corpus pins it.

## Compatibility and integration

- The phase-1 request JSON is unchanged and remains the default behavior.
  `synthesis` is an optional block whose **defaults are all off** — omitting
  it, or passing the default block, is byte-identical to phase 1:
  ```json
  { "maxNewOutputs": 0, "companionRecreations": false, "successorStates": false,
    "splits": false, "mints": false, "permuteOutputs": false }
  ```
  The fully-enabled block — what the flagship vault run declares — is:
  ```json
  { "maxNewOutputs": 2, "companionRecreations": true, "successorStates": true,
    "splits": true, "mints": true, "permuteOutputs": true }
  ```
- `request_from_map` (the map→hunt feed from #63) gains the same block so a
  mapped set is hunted with synthesis in one call — the vault is the first
  target that needs it.
- The CLI and report shapes are unchanged; `DrainReport` gains the synthesis
  caps and the enabled-degrees record.

## Non-goals, recorded

- Coverage-guided steering over cost hot spots — **phase 3** (synthesis
  multiplies the space; guidance is what makes it tractable in general).
- Concolic minimisation of synthesized witnesses — **phase 3**; delta-debug
  on hits only if the report plumbing gives it for free.
- Multi-step / multi-transaction chains — **phase 3+**. The vault question
  is deliberately scoped to a single transaction; a drain that needs a setup
  spend (e.g. moving an authorizer NFT into position first) is out of scope
  even in phase 2.
- Pricing / worth-weighted objectives; economic probes — separate spec.
- Any claim of completeness. Phase 2 widens the family; the verdict
  vocabulary exists because it is still a sample.

## Estimate

The output generator is the engine work; the oracle, objective, verdicts and
witnesses are phase 1's, untouched. Realistically one to two weeks:
generator + caps + report fields first (with the admin-refusal test, which
is deterministic from day one), then the vault run and corpus pinning, then
the map-feed synthesis flag. The schedule risk is probe count: synthesis
multiplies the space, and the caps are load-bearing — the report must make
truncation impossible to miss.
