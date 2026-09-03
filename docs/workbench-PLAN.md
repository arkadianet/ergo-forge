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
     `Filter`-predicate guard pattern). **Severity tiered 2026-09-02** by the
     receiver's root: context variable → Low (9/79), lambda element → Medium
     (9/79), else High (61/79). Same findings, honest alarm level; spec
     amended. **Val-held guards followed 2026-09-02** (multi-use `val ok =
     x.isDefined` — single-use vals are compiler-inlined): seed 242 → 236
     findings, mainnet unchanged. **`||`/negation closed 2026-09-03** by the
     falsity rule (right operand of `||`, else branch); corpora unchanged.
   - **P3b — spend hunt: DONE 2026-09-02** (design:
     `docs/superpowers/specs/2026-09-02-p3b-spend-hunt-design.md`).
     `hunt::hunt(tree_bytes, &opts) -> Hunt`: six probes (heights `base`,
     `base+1M`, `1` × attacker/preserve output), each a full consensus
     reduction with no proof and no context vars, through `eval_scenario` —
     no new evaluator entry. Verdicts `spendableByAnyone` / `movableByAnyone`
     / `requiresProof` (+ distinct residual propositions) / `notUnderProbes`;
     a synthetic SELF is reported so an errored register read cannot read as
     safe. Surfaces: `ergo-es hunt`, `POST /api/v1/hunt`, the UI's
     Spendability section (with a paste-the-box form, since the shell makes no
     outbound calls). **Measured on mainnet (279 trees): 0 spendable or
     movable by anyone, 259 require proof, 20 not under probes — 17 of those
     with every probe erroring on a register read against the synthetic SELF,
     3 genuinely failing on non-key conditions.** Deferred, recorded in the
     spec: register/context-var fuzzing, data-input probes, key → address
     presentation.
   - **P3c — cost hot-spots: DONE 2026-09-02.** `hot_spots::hot_spots(&[CostLine])`
     folds the `cost-trace` per-step trace into rows per operation (named
     via `inspect::opcode_name`, evaluator detail kept), ranked by JIT units
     with count and share; `ergo-es eval --hot-spots` prints the table.
     Pure fold, feature-free; the trace itself stays behind `cost-trace`
     (thread-local recorder, so per-evaluation safe). Carried limitation:
     labels are opcodes, not source spans — attributing cost to AST nodes
     needs the evaluator to tag steps with IR ids (node-side, alongside
     P5-B's preorder map). Not surfaced in ergo-web until then: an
     opcode histogram is a developer view, not a reader view.
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
   - **P4a — HTTP service + reader UI: DONE 2026-09-01.** `ergo-web` (axum +
     tokio) exposes `POST /api/v1/inspect` (address or ErgoTree hex →
     ErgoScript + audit findings) and `GET /api/v1/health`, plus a plain
     HTML/CSS/JS page — no bundler. Stateless, no outbound calls. The
     decompile runs inside `spawn_blocking` + `with_large_stack` on every
     request: measured on the deepest mainnet tree (46 levels), the read path
     overflows tokio's 2 MiB default and fits in 3 MiB, so the 16 MiB guard
     carries real load — and a committed test over that real tree aborts if
     the guard is removed. Note the guard is required despite the engine's
     depth caps: the caps bound node count, not frame width, and real
     contracts cost >10× more stack per level than synthetic arithmetic.
     **Extended 2026-09-02:** `POST /api/v1/hunt` (P3b) and `POST
     /api/v1/eval` — the scenario loop over HTTP, body = the `ergo-es eval`
     scenario JSON, response = verdict / cost / residual / trace — plus a
     Scenario panel in the reader, which closes the thesis' first row (dApp
     devs: compile → address → test scenarios) for the browser shell.
     The API is the stable boundary keeping a future WASM build viable —
     which stays blocked on `ergo-sigma`'s `panic = "unwind"` requirement
     (its AVL verifier uses `catch_unwind` to fail closed; `wasm32` is
     abort-only on stable Rust).
7. **P4b — the playground: DONE 2026-09-03** (design:
   `docs/superpowers/specs/2026-09-03-playground-design.md`). Write mode:
   editor → `POST /api/v1/compile` with **compile-time parameters** —
   `$name`/bare identifiers through the compiler's `ScriptEnv`, `"$name"`
   and all-caps tokens inside string literals by substitution — with a
   params form generated from `compile::scan_params` (and `// $name: Type`
   hints). Outputs: tree, addresses, the decompiled round-trip, findings,
   spendability, caret on positioned errors. Examples gallery: 86 files, of
   which 79 are the node corpus's real deployed contracts. **Measured:**
   19/79 corpus contracts compile with an empty env, 58/79 with auto-filled
   params (rest: 8 EIP-5 templates, 4 reference-parser rejects, wrong-type
   guesses). **Live mainnet** (344 unique trees from the newest 60 blocks):
   2 spendable by anyone, 312 require proof, 30 not under probes; 25 flagged
   / 201 findings; 332/344 round-trip byte-exact.
   **Engine bump 2026-09-03 (node #291 + #292):** EIP-5 templates
   instantiate through `ContractTemplate::apply` (declared defaults fill
   gaps; the params form prefills them); the lift takes shared IR ids from
   `ergo_ser::opcode::preorder` (`Lifted::ir_ids`, `Finding::ir_id`), and
   the compile route positions findings in the authored source through the
   compiler's `SourceMap` — the reader selects the cited range in the
   editor. Positions are start offsets (carets, not ranges) per P5-A.
8. **P4c — contract test suites: DONE 2026-09-03** (design:
   `docs/superpowers/specs/2026-09-03-contract-tests-design.md`). A
   `contract.test.json` (contract + named scenarios with expected verdicts)
   runs through `testsuite::run`; `ergo-es test` is the CI entry point
   (non-zero exit on any failing case), `POST /api/v1/test` and a Tests
   panel in the playground (export the suite from the editor's contract).
9. **P4d — chain data in: DONE 2026-09-03.** `EXPLORER_URL` is the one
   outbound dependency, off by default (the self-hosted promise holds;
   `/api/v1/config` says which mode). `POST /api/v1/lookup` fetches a box by
   id or an address's unspent boxes plus the chain height in the scenario
   box shape, registers as `{"type":"raw"}` serialized constants the engine
   re-parses. The reader's "Fetch from chain" fills SELF and the height and
   re-hunts — the spendability answer for the real box.
10. **P4e — Build mode, recipes, files: DONE 2026-09-03.** A recipe
    library of EIP-5 templates (`examples/contracts/recipes/`: time lock,
    inheritance, 2-of-3, refundable payment, price gate, burn) whose docs
    are the wizard's questions; `scan_params` carries `@param` descriptions
    and `template_doc` the contract's; a `SigmaProp` parameter accepts a P2PK
    address; `/api/v1/config` reports the chain height so a date becomes a
    height client-side. Build mode: pick, answer, get an address with a
    spendability verdict and a plain-language summary; share link; project
    zip (`contract.es` + `params.json` + `contract.test.json` + README)
    that `ergo-es test` runs unchanged; open `.es`/project files into Write.
    Raw `.es` at `/api/v1/examples/{id}.es`.
11. **P4f — transaction validation: DONE 2026-09-03.** `txcheck::check`
    runs every input's script of an unsigned node-format transaction in
    the real context (new scenario `selfIndex`: SELF at its index, inputs
    in order; outputs; data inputs; the input's extension as raw context
    vars) and checks ERG/token conservation (minting with the first
    input's id allowed). Signatures are not checked — `needsProof` inputs
    count as signatures needed. `ergo-es validate-tx`, `POST
    /api/v1/validate-tx` (missing boxes fetched from the explorer when
    configured), a Validate section in Read.
12. **P4g — the composer: DONE 2026-09-03.** `compose::compose(spec,
    values)`: spending paths (who: anyOne/anyOf/allOf/kOf; conditions:
    after/before/payTo/keepHere/oracleAbove) → readable ErgoScript with
    `$name` params; with values, a generated suite whose expectations come
    from the composer's own model (satisfied paths → pass / needsProof
    with residual assertions / fail), so running it checks the assembly
    against the evaluator. Surfaces: `ergo-es compose`, `POST
    /api/v1/compose`, and "Combine rules yourself" in Build with a checks
    table on the result. Thirteen recipes precede it as the tested clause
    library; three more on 2026-09-03 (auction with bidder refunds, NFT
    sale with a creator royalty, HTLC with SHA-256 for cross-chain swaps),
    25 suite cases. Both output-reading recipes first indexed OUTPUTS(0)
    eagerly and errored on an empty output list; the suites caught it.
    - **Vocabulary widened, 2026-09-03.** The condition set now covers what
      a script can see: block timestamp windows, box age, input/output
      counts, a general box rule (this box / an output / an input / a data
      input; by index, any, or all; script, value, share of SELF.value,
      token with amount, no tokens, tokens preserved, R4..R9 typed
      comparisons incl. `eqHeight` and `eqSelf`), context variables,
      hash preimages (blake2b256/sha256, with a `witness` for the checks),
      token gating over inputs, the miner key, and paid totals via a fold.
      The model got a real world (self box with tokens/registers/creation
      height, extra inputs, data inputs, vars, timestamp, miner); violation
      cases break the most specific requirement; contradictory paths are
      refused rather than passing vacuously. The UI's "Combine rules
      yourself" lists 24 conditions in seven plain-language groups plus an
      advanced box-rule form; verified headless (3 paths, 15 checks green).
      Arithmetic and conservation added the same day: a box may hold at
      most X less than SELF, a token is conserved across outputs (sum
      equality via flatMap+fold; a fold nested in a fold lambda does not
      type-check in the compiler), and an output mints a token named after
      the first input (EIP-4). Still outside the composer: AVL-tree
      proofs, arbitrary arithmetic and folds beyond sums, sigma protocols
      beyond keys — Write mode.
13. **P4h — protocol templates: DONE 2026-09-03.** Multi-box systems in
    the shape of deployed contracts, validated by property-style suites
    whose expectations come from an independent Python model of the rules
    (`examples/tests/gen/`), swept over prices/amounts and pinned at the
    one-unit boundaries, run through the node's reducer in CI. AMM: a
    constant-product ERG/token pool (Spectrum N2T shape, one script) — 69
    cases; a swap order — 6. Bank: AgeUSD/SigmaUSD shape with oracle data
    input, fee, reserve-ratio window — 47 cases across six ERG prices,
    under- and over-collateralised states. Both models and both scripts
    agree on every case; the boundaries (one unit over the curve, one
    nanoERG short of price plus fee) are the teeth.
14. **P5 — positions and editor surface** (node-side, `arkadianet/ergo`). Lets
   tree-level audit findings project back onto authored source — squiggles,
   hovers, eventually LSP.
   - **A — DONE 2026-08-31** (`compiler/source-positions`, `8479a58`): every
     `TypedExpr` carries `pos`; `CompileError::pos()` is real for `Type`.
     Verified additive — oracle grading compares verdict + exception class, not
     position, and all 1015 `ergo-compiler` tests pass unchanged. Scala *does*
     carry typer positions (`Value._sourceContext`, ~75 cited sites in
     `SigmaTyper.scala`), so the old `0` was a gap, not parity.
   - **B — DONE 2026-09-03** (node PR #292): `compile_with_source_map` +
     `ergo_ser::opcode::preorder`, the single shared walk both sides take ids
     from. Implemented as emit-time origin recording resolved by top-down
     alignment against the final tree (six rewrite passes run after emit;
     the design doc records the departure). Consumed here: `Lifted::ir_ids`,
     `Finding::ir_id`, positioned findings on `/api/v1/compile`.
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
- CLI shell: `ergo-sandbox/src/bin/ergo-es.rs` (`compile` / `params` /
  `eval` / `decompile` / `roundtrip` / `audit` / `hunt` / `test`).
- Web: `ergo-web` (workspace member, P4a) — axum service over `ergo-sandbox`
  plus the no-bundler UI in `ui/`. The WASM build originally planned here is
  blocked (see P4); the HTTP API is the stable boundary in the meantime.

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
