# Contract test suites

> Design record, 2026-09-03. The dApp-dev loop, made repeatable.

## Why

The Scenario panel runs one context at a time. A contract has many paths
(refund after deadline, spend with the right key, reject an underpaid
output) and a developer wants to keep all of them and re-run them on every
change — in the playground and in CI. That is a test suite, and the engine
already answers each case; what is missing is the file, the runner, and the
table.

## The file

One JSON document, `contract.test.json` by convention:

```json
{
  "source": "sigmaProp(HEIGHT > $unlockHeight)",
  "params": { "unlockHeight": { "type": "Int", "value": 1000 } },
  "network": "mainnet",
  "scenarios": [
    { "name": "locked before the height", "expect": "fail", "height": 999 },
    { "name": "unlocked after",           "expect": "pass", "height": 1001 }
  ]
}
```

- `source` (+ `params`, compile-time constants as on `/api/v1/compile`) or
  `tree` (hex): the contract under test, compiled ONCE for the suite.
- `scenarios[]`: each is a **scenario** (the `ergo-es eval` schema — `height`,
  `selfBox`, `inputs`, `outputs`, `dataInputs`, `contextVars`, `proof`…)
  plus `name` and `expect`. `expect` is a verdict: `pass`, `fail`, `error`,
  `needsProof`, `proofAccepted`, `proofRejected`. A scenario may not name its
  own `source`/`tree` — the suite's contract is the point.

## The runner

`ergo_sandbox::testsuite::run(&Suite) -> SuiteResult`: compiles the contract,
then evaluates every scenario against it. Each `CaseResult` carries the
expected and actual verdicts, `passed`, the runtime error text when the
script threw, the residual proposition, and cost. Marshalling errors in a
scenario (bad register value) fail that case with the error text rather
than aborting the suite; a contract that does not compile fails the suite.

Surfaces:
- `ergo-es test <suite.json>` — a table, one line per case, and a non-zero
  exit when any case fails. This is the CI entry point.
- `POST /api/v1/test` — body is the suite, response is the `SuiteResult`.
- UI: a Tests panel under Write. The suite editor holds the `scenarios`
  array; the contract is the editor's source and params. Run, read the
  table, fix, run again.

## Not in scope

- Assertions beyond the verdict (cost ceilings, residual key). The result
  carries them; asserting on them is a later increment once someone asks.
- Property/fuzz generation. The spend hunt is the sampling instrument.
