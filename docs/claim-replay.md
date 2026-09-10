# P05: offline transaction and declared-property replay

`ergo-es replay <case.json> --json` reruns the full pinned node transaction
validator, then evaluates `recognized-attacker-receipts-v1` over the accepted
transaction's resolved inputs and outputs. It never searches, signs, resolves a
source URL, fetches state or broadcasts. The claim is **node-accepted under these
supplied premises and violates this declared property**. It is not a finding's
severity, a historical-state proof, or an unconditional deployed-exploit claim.

For example, from the repository root:

```sh
CARGO_TARGET_DIR=./target-p00 cargo run --release -p ergo-sandbox --bin ergo-es -- replay ergo-sandbox/tests/fixtures/evidence/claim-vectors/use-incident.fixture --json
```

The strict version-1 `ReplayBundle` contains `execution` (P03's complete
`ValidationRequest`) and `property`: its version, every spending input's exact
box ID and declared role in transaction order, recognized recipient public keys,
and the explicit release objective. Every field participates in the bundle
fingerprint. Missing/unknown policy, missing bindings, wrong companion identities
and unsupported authorization forms produce no confirmed claim. A legitimate
change to an allowance changes the question and fingerprint; it may change the
answer. Roles, recognized recipient keys and authorization terms remain declared
premises, not independently authenticated ownership or protocol intent.

`ConfirmedViolation` is private to the acceptance/property path and has no
serde constructor. The replay JSON includes the full bundle, fingerprint, node
report, property version, accounting, input provenance and scope. A stored report
cannot deserialize as a bundle; replay its `bundle` to derive a fresh answer.
There is no accepted-flag import or claim cache. `confirmed-violation` and
`accepted-nonviolating` exit zero. Node rejection, incomplete premises, unsupported
property and malformed input exit nonzero. Unsupported property can still carry
`nodeValidated: true`, explicitly distinguishing acceptance from property evidence.

The existing accounting moved to `drain/accounting.rs` with its function bodies
unchanged. The hunt calls the same functions as before. Replay projects only
exact accepted value, token and script fields into that accounting; these
projection objects never enter a reducer or box serializer. Data inputs are
validated by the node and fingerprinted in the execution bundle but contribute
no spending funds. Replay does not use hunt caps, synthesis, NFT-based role
inference or a generated witness. Values outside accounting v1's representation
return unsupported rather than being narrowed silently.

The separate policy-registered `claim-manifest.json` binds all request/source
files by SHA-256, node revision, family, property version, publication eligibility
and expected identity/status. It preserves the completed P03/P04 manifests. The
new measured set has three distinct accepted transactions, two positive claims
in two families, one accepted nonviolating control and zero claims on negative
controls. One case has source-recorded inputs; three have hypothetical inputs.
All have explicitly supplied context, not authenticated historical state. These
measurements are separate from the frozen preflight corpus and baseline scoreboard.

The USE fixture uses source-recorded inputs, proofs, block context, the incident
epoch's active parameters, mainnet rule configuration and preceding headers.
[Recovery artifacts and derivations](p05-recovery/README.md) record the sources.
The acceptance test re-derives the complete request and validates every preceding
transaction to obtain prior block cost; the original hypothetical candidate and
relabeled substitutions cannot satisfy the public-incident gate. Header and
transaction commitments are checked through pinned node primitives. Historical
UTXO membership and canonical-chain authentication remain outside this claim.
The original [stop](P05-REPORT.md) is retained, with its reopening governed by
[P05-DECISION.md](P05-DECISION.md); no threshold was lowered.

The authored fixed-rate sale pair removes only the seller-destination check in
the mutant. An unpaid mutant spend violates the declared rate; a paid fixed spend
is node-accepted and nonviolating. An unpaid fixed counterfactual is node-rejected.
Each changed script gets newly serialized hypothetical boxes and IDs. Funding
proofs are generated with P04 using a public test-only key; no scalar enters the
bundle. Tests recompile both sources with the pinned compiler and check script
identity. These are new authored fixtures, not edits to the measured mutant rows.

The offline recovery producer is `docs/p05-recovery/derive`. It records canonical
construction and can explicitly write the recovered USE fixture; it is not an
automatic gate repair. The gate executes its read-only derivation helper. The
library and CLI need no source checkout or network at replay time: complete
execution material, provenance references and archived transaction JSON are in
the bundle. To audit provenance derivations as well as replay execution, share
the accompanying recovery directory and pinned engine revision.
