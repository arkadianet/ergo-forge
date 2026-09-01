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
- compile + templates (`ergo-compiler` M1–M4; 95/110 byte-parity). **Positions are
  partial:** `CompileError::pos()` returns real offsets for `Parse`/`Bind` only;
  `Type`/`Root`/`Emit`/`Write` return `0` because `TypedExpr` carries no positions
  (typecheck.rs:101, E12). Type errors — the class a playground user hits most —
  are not underlinable today. See P5.
- evaluator reachable standalone (`EvalBox`, `ReductionContext::minimal`, `CostTrace`)
- node API design for inspect/execute/cost/simulate/explain/diff (tooling-api doc T1–T5)
- dashboard UI tiers (templates / editor / safety rails) — compiler-UI doc

Built since (see Phases for the verified records):
1. ~~**Sandbox crate**~~ — **DONE (P1).** Context construction, eval session,
   cost trace, WASM-clean deps.
2. ~~**Decompiler**~~ — **DONE (P2).** ErgoTree bytes → readable ErgoScript; the
   long pole, and still the capability nothing else in the ecosystem has.
   Graceful degradation to honest `<…>` placeholders for hand-built/soft-fork
   trees is in place.

Still missing (the actual build list):
3. ~~**Public lifted AST**~~ — **DONE (P2.5).** `decompile::lift_tree` exposes
   `Node { id, kind }` to lints; ids are lift-local pending `ergo_ser::preorder`.
4. **Audit layer** — static lints over the lifted tree (height guards, `anyOf`
   shadowing, trust assumptions, unchecked `get()`), scenario fuzzing
   ("spendable by anyone?" hunts) over the sandbox, cost hot-spot reports.
5. **Standalone browser shell** — the classroom for non-node-operators;
   WASM bindings over the engine.
6. **Positions** — type errors carry no source offset, so the editor cannot
   underline the most common failure. Node-side work; blocks the LSP story.

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
   constants. Bar measured with `ergo-es roundtrip`:
   - **seed (110 vectors, v3/testnet; 87 compile with an empty env):
     73 byte-exact · 11 diff · 0 raw · 3 err**
   - **mainnet (279 unique trees): 270 byte-exact · 6 diff · 2 raw · 1 err**
   - Two failure classes: `raw` = decompiler placeholders (honest `<…>` for
     constructs with no lift yet); `diff`/`err` = re-renderings the compiler
     refuses — and for every `err`, **Scala's reference compiler rejects the
     identical source** (verified 2026-08-31 via the JVM TyperOracle,
     sigma-state 6.0.2), so there is no Rust/Scala divergence: 17 `diff` from
     upstream constant folding; 3 fold-in-lambda-apply errs (the reference
     binder inlines `def` bodies into `FuncApply(FuncValue, arg)` wire shapes
     that its own front-end cannot re-parse; Rust's `assignType(Fold)` is the
     twin of Scala's `TyperException`); 1 fold-lambda typing err. Full table
     with repros in `ergo-sandbox/README.md`.
   - `cargo test` pins the bar off a committed fixture
     (`tests/fixtures/compile_corpus_subset.json`, 73 vectors);
     whole-corpus floors run when the node checkout is a sibling.
   - **Stack budget:** the lift recurses and debug frames are wide — 46 levels
     need ≈3 MiB. `decompile::with_large_stack` gives headroom; the lift is
     bounded by `MAX_LIFT_DEPTH` so it degrades instead of overflowing. The
     WASM/HTTP shell must use the wrapper (or an iterative rewrite).
4. **P2.5 — expose the lifted AST (DONE 2026-09-01)** (prerequisite for P3; design:
   `docs/superpowers/specs/2026-08-31-lift-target-ast-design.md`).
   `decompile.rs` splits into `decompile/{ast,lift,print,mod}.rs`; the lifted
   node types go public (`L` → `Node`), precedence moves out of the AST into the
   printer, `lift()` returns `Lifted { node, raw_placeholders, truncated }`, and
   every node carries `ir_id` (its IR preorder index) so source citation is a
   later lookup rather than an AST change. Behaviour-preserving: every existing
   test must pass with no edits to expected values. Small, and it unblocks
   everything in P3.
   Shipped: `decompile/{mod,ast,lift,print}.rs`; `pub Node { id, kind }`,
   `pub NodeKind`, `pub Stmt`, `lift_tree() -> Lifted`, `print(&Node)`.
   Precedence derives from the operator symbol in `print::prec_of`. Ids are
   lift-local pending `ergo_ser::preorder` (P5-B).
5. **P3 — audit layer.** Lints run on the **lifted tree**, not on parsed source —
   authored source reaches them via `compile_source` then `lift`, the same path
   an on-chain address takes. One lint suite serves both inputs, and it audits
   what consensus will actually execute. A `Raw` placeholder in the tree is a
   confidence signal: an audit over a tree containing one degrades its verdict
   rather than reporting clean.
   - **P3a — static lints: DONE 2026-09-01.** `audit::audit(&Lifted) -> Audit`,
     lints as `fn(&Node) -> Vec<Finding>` in a const registry. First lint:
     `unchecked-get` (High). `Completeness` reports when the lift left raw
     placeholders, so an audit over a partly-lifted tree cannot read as clean.
     Measured flag rate on mainnet: 13/279 trees flagged (4.7%), 79 findings,
     75/79 hand-verified real (4 false positives, all the untracked
     `Filter`-predicate guard pattern).
   - P3b (scenario fuzz) and P3c (cost hot-spots) are separate — neither
     depends on the AST.
   - static lints over `Node` (height guards, `anyOf` shadowing, unchecked
     `get()`, trust assumptions)
   - scenario fuzz over the sandbox ("spendable by anyone?" hunts)
   - cost hot-spot views off the existing `cost-trace`
   - REST surfaces fold into tooling-api T1–T5 where they overlap
6. **P4 — WASM + browser workbench**; templates gallery (UI-doc Tier 1) reused.
   Two carried constraints: the lift and the printer both recurse (≈3 MiB at 46
   levels), and `wasm32` has no threads — so `with_large_stack` degrades to
   inline and worker stacks are small. Either budget the stack or take the
   iterative rewrite. This is also the point the `ergo-decompile` crate
   extraction pays for itself (browser builds pull decompile without
   `eval`/`scenario`); mechanical once P2.5 has landed the module split.
7. **P5 — positions and editor surface** (node-side, `arkadianet/ergo`). Lets
   tree-level audit findings project back onto authored source — squiggles,
   hovers, eventually LSP.
   - **A — DONE 2026-08-31** (`compiler/source-positions`, `8479a58`): every
     `TypedExpr` carries `pos`; `CompileError::pos()` is real for `Type`.
     Verified additive — oracle grading compares verdict + exception class, not
     position, and all 1015 `ergo-compiler` tests pass unchanged. Scala *does*
     carry typer positions (`Value._sourceContext`, ~75 cited sites in
     `SigmaTyper.scala`), so the old `0` was a gap, not parity.
   - **B — designed, not built** (`docs/ergoscript-compiler-source-map-design.md`
     in the node): emit-time IR-node ↔ source-offset map. **Its keying contract
     needs one change before implementation** — indices must come from a single
     shared `ergo_ser::preorder` walk, not be computed independently on each
     side. Today's lift skips subtrees at `MAX_LIFT_DEPTH`, which would silently
     misalign every citation after the first deep contract. See the amendment in
     `docs/superpowers/specs/2026-08-31-lift-target-ast-design.md`.
   - Carried limitation: the parser records start offsets only, so P5 yields
     carets, not underlines. Ranges need end-offset capture in `ast.rs` +
     `parse/*` first.

## Crates

- `ergo-sandbox` (workspace member): eval/compile/decompile session APIs.
  Deps: ergo-ser, ergo-sigma, ergo-compiler, ergo-primitives. No tokio/redb.
  The decompiler lives at `ergo-sandbox/src/decompile.rs`, becoming
  `decompile/{ast,lift,print}.rs` at P2.5. The separate `ergo-decompile` crate
  originally planned is **deferred, not cancelled**: it buys nothing until a
  browser build wants decompile without `eval`/`scenario` (P4), and the P2.5
  module split makes extracting it mechanical when that day comes.
- CLI shell: `ergo-sandbox/src/bin/ergo-es.rs` (`compile` / `eval` /
  `decompile` / `roundtrip`).
- Web: separate repo later; consumes WASM builds of ergo-sandbox.

## Open decisions

- ~~Byte-exact vs canonical re-serialization tolerance~~ — **resolved by P0**:
  394/394 real trees re-serialize byte-exact; the bar is byte-exact.
- ~~Whether the audit lints operate on the lifted AST or a separate crate~~ —
  **resolved 2026-08-31** (design doc above): lints operate on the lifted AST,
  made public in place by P2.5. No separate crate until P4 needs the dependency
  split. `ergo_compiler::ast::Expr` was rejected as the lift target — it is
  oracle-pinned to the Scala parser, and a `Raw` variant the parser can never
  emit would weaken that parity artifact.
- Whether the P5 source map lives in `ergo-compiler`'s emit phase or a parallel
  side-table — decided when span threading starts.
