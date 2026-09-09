# P03: full transaction validation against supplied premises

`evidence::validate::validate(&ValidationRequest)` invokes the pinned node's
`ergo_validation::tx::validate_transaction` raw-byte pipeline. It does not call
`txcheck`, a parsed shortcut, or a copied list of consensus predicates. A
successful call returns `AcceptedExecution`, holding the node's private
`CheckedTransaction`. Its fields are private and it has no deserializer or
public constructor. `report()` exports inspectable JSON; importing that JSON
cannot manufacture the capability.

The request has `formatVersion: 1`, a P01 `case`, exact `transactionBytes`, and
explicit `Premise` values for `parameters`, `networkRules`, `blockContext`,
`headers`, `localPolicy`, and `priorBlockCost`. Every protocol-parameter field
and block-context field, including `activatedScriptVersion`, is mandatory.
There are no mainnet, activation, header, miner-key, or zero-prior-cost defaults.
An explicitly empty header array is a supplied hypothesis, not an inferred
historical header window. Unknown or malformed premises stop before invocation.
The adapter does not establish that supplied parameters or activation data were
voted into effect at the stated height.

`networkRules` explicitly enables re-emission with its activation height, token
ID and pay-to-reemission tree, or explicitly disables it with a description and
reason. These are supplied rule configurations, not authenticated network
identities. The adapter never silently substitutes the node's default disabled
rules. `localPolicy.maxTransactionSize` remains local policy, not a consensus
claim. `priorBlockCost` uses block-cost units and initializes the node's single
cost accumulator before validation; no per-input accumulator reset is added.

The case's `boxes` premise is the supplied UTXO snapshot. Every record goes
through P02's strict full-box import; lookup keys come from node-derived IDs.
Duplicate snapshot members are rejected. An explicitly empty snapshot reaches
the node and can produce its missing-UTXO rejection; a missing snapshot is an
incomplete premise. Source-recorded boxes still do not prove historical UTXO
membership. Independently-checked origin claims for context/rules are unsupported
and rejected rather than accepted from JSON.

If the case already carries context, it must exactly match the typed block
context, including provenance. A pre-existing P02 transaction attachment must
match the request bytes. The full raw-byte entry remains available for diagnosing
noncanonical transactions without P02's strict import rejecting them first.
Source identity, target, roles, objectives and other case assumptions are retained
in the report but are not authenticated or proved by transaction acceptance.
P03 makes no property claim about them.

Accepted reports say `node-accepted`, `nodeValidated: true`, and
`pipelineInvoked: true`, with the node revision, node-derived transaction ID,
total block cost, full original request and its fingerprint. This means **accepted
against the supplied state and rules**. It does not mean the state is real, the
transaction was historically accepted, a security property was violated, a block
would validate, or the transaction will be included. Header-chain verification,
PoW, state authentication and parameter-history verification are outside this
transaction entry point.

A node rejection says `node-rejected`, `pipelineInvoked: true`, and
`nodeValidated: false`; the original node diagnostic and a coarse stage label
are retained with the request. A missing/invalid premise instead says
`incomplete-or-invalid-premises`, `pipelineInvoked: false`. Stage labels classify
errors for display; they never decide acceptance. Local-policy/deserialization
errors share an explicitly named diagnostic category. Re-import the report's
`request` as `ValidationRequest` and call `validate` again; stored acceptance flags
are neither request fields nor trusted inputs.

The [manifest](../ergo-sandbox/tests/fixtures/evidence/manifest.json) pins nine
public authored vectors and hashes every request file. It includes the eight
ROADMAP cases plus an enabled re-emission rejection. Expected results were
captured directly from the full node path. Tests verify all hashes and revisions
before executing cases, and check controls that remove the rent extension, remove
prior accumulated cost, and explicitly disable the re-emission rule. Required
IDs and the minimum count are read from ROADMAP's policy block.

P03 adds a library capability only. Legacy HTTP/CLI verdicts, preflight, Play,
search, signing and browser storage remain unchanged. No P04 signing operation
or P05 property-claim producer is implemented. Frozen scoreboard zeros remain
measurements at their recorded baseline revision; these new hypothetical codec/
validation vectors do not rewrite those numbers.
