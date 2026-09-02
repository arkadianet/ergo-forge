# P4b — the playground: write, parameterise, compile, test

> Design record, 2026-09-03. Turns the reader into an ErgoScript playground.

## Why the reader is not a playground

The reader starts from an address. A playground starts from source. The 79
real deployed contracts vendored in the node's corpus
(`test-vectors/ergoscript/corpus`: Dexy, HodlCoin, ChainCash, Rosen Bridge,
CrystalPool, EIP-5 examples) show what "source" means in practice: only 19
compile with an empty environment. The other 60 use **compile-time
parameters** — `$oracleNFT`, `$minerFee`, `$phoenixFeeContractBytesHash`,
bare names like `PoolNFT` — which each project substitutes before compiling.
A playground that cannot take parameters cannot open a real contract.

## Parameters

Two mechanisms, because the corpus uses two:

1. **Environment constants.** `$name` and bare `Name` identifiers resolve
   through the compiler's `ScriptEnv` (the same path as the node's
   `/script/p2sAddress` `keysToEnv`). Values arrive as the workbench's typed
   JSON (`{"type": "Long", "value": 5}`), parsed by the scenario parser and
   mapped to `EnvValue`. A `SigmaProp` pubkey becomes a real `ProveDlog`.
2. **Textual substitution inside string literals.** `fromBase64("$poolNFT")`
   needs the base64 text in the string, not a constant. A parameter of type
   `String` is substituted textually wherever `"$name"` appears in a string
   literal; `Coll[Byte]` values are substituted as hex inside `fromBase16`
   strings. Nothing else is touched textually.

**Discovery.** `compile::scan_params(source)` lists every `$name` the source
uses (outside comments), with a type hint when the source carries the corpus
convention `// $name: Type`. The UI renders a form from it before the first
compile; missing parameters are a structured error, not a typer message.

EIP-5 `@contract def` templates (5 corpus files) are detected and reported
as unsupported for now: the compiler produces a `ContractTemplate`, but
applying parameter values to placeholders is not exposed yet.

## Surfaces

- **Engine:** `compile::compile_with_params(source, &params, tree_version,
  network)`, `compile::scan_params(source) -> Vec<ParamNeed>`.
  `CompileError::pos()` offsets are carried through so the UI can point.
- **Web:** `POST /api/v1/compile` `{source, network?, treeVersion?, params?}`
  → `{treeHex, p2s, p2sh, source (round-trip), completeness, findings,
  params: [{name, typeHint, supplied}]}`; on failure 400 with
  `{code: "compile_error", message, offset?, missingParams?}`.
  `GET /api/v1/examples` → `[{id, group, name}]`; `GET /api/v1/examples/{id}`
  → `{id, source, params}`. Examples are files under `EXAMPLES_DIR`
  (default `examples/contracts`, vendored in this repo with an origin note).
- **UI:** a Write mode alongside Read. Editor, params form (auto-populated
  from the scan, type from the hint or a picker), Compile. Outputs: tree,
  addresses, round-trip source, findings, spendability, and a caret line
  under the editor on a positioned error. The Scenario panel takes the
  editor's source and params when the scenario has neither `tree` nor
  `source`.
- **Corpus tooling:** `ergo-es audit|hunt --trees file.json` (a JSON array
  of tree hex) so any ad-hoc corpus — including live trees pulled from the
  explorer — runs through the same measurement.

## Verification

- Engine tests: env parameters reach the tree; string substitution; scan
  finds names and hints, ignores comments; missing params are structured.
- The 79-file corpus: compile rate with an empty env (19) vs. with a
  generated parameter set (every scanned param filled with a typed default)
  — recorded in the plan.
- Live chain: trees from the newest N mainnet blocks through inspect /
  audit / hunt; tallies recorded.
