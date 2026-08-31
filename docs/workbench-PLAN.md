# ErgoScript Workbench — plan

> Working notes (dev-docs is gitignored). Positions the workbench against the two
> existing design docs; does not replace them.
> Siblings: `ergoscript-tooling-api.md` (node engine API, `/api/v1/script/*`),
> `ergoscript-compiler-ui-design.md` (operator-dashboard UI tiers).

## Thesis

One engine, three shells, three audiences:

| Audience | Need | Shell |
|---|---|---|
| dApp devs (TS/Rust) | compile → address → **test scenarios** | CLI + REST + browser |
| Learners / auditors | **read** live contracts (decompile), inspect, cost | browser (paste address/box-id/tree hex) |
| This project | dogfood `ergo-compiler`/`ergo-sigma`; parity oracle, productized | CLI + CI |

The engine reuses exactly two consensus primitives — `ergo_compiler::compile` and
`ergo-sigma`'s reduce/cost path (`verify_spending_proof_with_context_and_cost`,
`reduce_expr_with_cost`) — per the tooling-API doc's one-primitive rule. No second
interpreter, no second compiler. The UI shell follows the vanilla-JS/no-bundler
constraints from the compiler-UI doc.

## What exists vs. what's missing

Exists (engine-level, grounded):
- compile + errors-with-spans + templates (`ergo-compiler` M1–M4; 95/110 byte-parity)
- evaluator reachable standalone (`EvalBox`, `ReductionContext::minimal`, `CostTrace`)
- node API design for inspect/execute/cost/simulate/explain/diff (tooling-api doc T1–T5)
- dashboard UI tiers (templates / editor / safety rails) — compiler-UI doc

Missing (the workbench's actual build list):
1. **Sandbox crate** — context construction sugar + eval session + cost trace
   exposed to non-consensus callers (thin re-plumb of validation's
   `tx/script/mod.rs` wiring; WASM-clean deps only).
2. **Decompiler** — ErgoTree bytes → readable ErgoScript. The long pole. Not
   covered by any existing doc (tooling-api's `inspect` ships opcode dump +
   typed s-expr only; UI doc's "disassemble" is the same). Needs IR → lifted
   AST → pretty-print, normalization via cast-fold/CSE, graceful degradation to
   a structural view for hand-built/soft-fork trees.
3. **Audit layer** — static lints over the lifted AST (height guards, anyOf
   shadowing, trust assumptions, unchecked `get()`), scenario fuzzing
   ("spendable by anyone?" hunts) over the sandbox, cost hot-spot reports.
4. **Standalone browser shell** — the classroom for non-node-operators;
   WASM bindings over (1)–(3). Later.

## Verification bar (decided)

- **Decompiler v1: byte-exact recompilation on known-provenance corpus**
  (compile_seed vectors + 79-file contract corpus). Rationale: compile is already
  oracle-tested, so `decompile → recompile → byte-identical` is checked against
  proven machinery — a divergence is precisely a decompiler bug. Sub-second CI.
- **Unknown provenance (mainnet trees): graded badge**, never a gate —
  `exact` / `structural` (lifted view, no round-trip claim) with the reason.
- **Semantic equivalence (bounded scenario sampling)** belongs to the audit
  layer, not decompiler CI.

## Phases

1. **P0 — spike (DONE 2026-08-31, `ergo-compiler/examples/decompile.rs`):**
   structural printer over `ergo_ser`'s `Expr` IR. Results across 394 real
   trees (110 compile-seed vectors + 279 unique mainnet trees from the 213-tx
   diff corpus + 5 hostile `failing_tree` files):
   - **0 parse failures, 0 unmapped opcodes** (61–67 distinct opcodes observed,
     all inside the evaluator's documented set);
   - **100% byte-exact re-serialization** (`write_ergo_tree` == original bytes
     on every tree, hostile ones included) → the v1 bar stays **byte-exact**;
     no canonical-tolerance relaxation needed (open decision resolved);
   - max depth 46 (bound is 110); heavy hitters: ValUse, ConstantPlaceholder,
     ByIndex, ValDef, EQ, If, BinAnd, SelectField;
   - spike-quality output is already *readable* — mainnet #0 is the emission
     box script (height guards + SubstConstants miner-PK + demurrage
     arithmetic), #1 a timelock+PK, #2/#3 P2PK.
   **Verdict: decompiler v1 ≈ 2–3 weeks**, not 2 months. Remaining work is
   presentation, not coverage: SSA ValDefs → source `val` bindings, method/
   property id → name tables (reuse `ergo-compiler` typer/predef_ir tables),
   infix sugar for operators, and the decompile→recompile→byte-compare CI
   harness.
2. **P1 — sandbox crate + CLI `eval`** (contract + scenario JSON → verdict +
   cost). Vertical slice; everything later is a new front on it.
   **DONE 2026-08-31:** `ergo-sandbox/` in the workspace — `scenario`
   (JSON model + typed-value parser), `eval` (scenario → `ReductionContext`
   → bounded-cost consensus reduce → `EvalOutcome{verdict, cost, trace}`),
   `compile` (thin `ergo_compiler::compile` wrapper), `inspect` (structural
   printer, superseding the P0 example — deleted). CLI `ergo-es`:
   `compile` / `eval` / `decompile` (incl. `--seed`/`--mainnet` corpus recon).
   13 integration tests + doctest green; clippy `-D warnings` clean with and
   without `cost-trace`. Verdicts: PASS / FAIL / ERROR / NEEDS-PROOF /
   PROOF-ACCEPTED / PROOF-REJECTED; proof path runs
   `verify_spending_proof_with_context_and_cost` (the exact block-validation
   function). Known compiler-scope gap surfaced: `getVar` predef / Select on
   SContext not yet emittable (falls in the compiler's remaining 15/110
   vectors) — scenario context vars are accepted and stored, end-to-end
   GetVar tests pending that emit work.
3. **P2 — decompiler v1** with the verification bar above; powers
   `inspect`-upgrade and the "read live contracts" loop (paste address/box-id →
   readable source).
   **DONE 2026-08-31 (`ergo-sandbox/src/decompile.rs` + `method_names.rs`):**
   IR lift + pretty-printer — SSA ValDefs → `val` bindings, method/property
   id tables (201 entries, extracted from the oracle-pinned compiler method
   tables), infix sugar, fold tuple-lambda unwrap, network-aware PK
   constants. Round-trip tally over the node's oracle-graded corpora:
   - **seed (110 compile vectors, v3/testnet): 66 byte-exact**, 13 diff
     (compiler-side fold collapses of trivial sigma props — semantically
     equal), 1 raw, 7 err (5 from a confirmed **ergo-compiler bug**: fold
     inside an operator operand fails `assignType(Fold)` — filed upstream)
   - **mainnet (279 unique trees): 259 byte-exact**, 1 diff, 15 raw
     placeholders, 4 err
   CLI: `ergo-es roundtrip --seed | --mainnet | <hex>`; detailed harness in
   the `corpus_roundtrip` example.
4. **P3 — audit layer** (lints + scenario fuzz + cost views); REST surfaces fold
   into tooling-api T1–T5 where they overlap.
5. **P4 — WASM + browser workbench**; templates gallery (UI-doc Tier 1) reused.

## Crates

- `ergo-sandbox` (new, workspace): eval/compile session APIs. Deps:
  ergo-ser, ergo-sigma, ergo-compiler, ergo-primitives. No tokio/redb.
- `ergo-decompile` (new, workspace): IR lift + pretty-print. Deps: ergo-ser
  (+ ergo-compiler for type-term strings).
- CLI shell: `examples/` or a small `ergo-es` bin — decided at P1.
- Web: separate repo later; consumes WASM builds of the two crates.

## Open decisions

- ~~Byte-exact vs canonical re-serialization tolerance~~ — **resolved by P0**:
  394/394 real trees re-serialize byte-exact; the bar is byte-exact.
- Whether the audit lints live in `ergo-decompile` (operate on lifted AST) or a
  third crate — decided when the lifted AST shape exists.
