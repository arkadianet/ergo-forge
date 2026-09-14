# Incident scaffold

From the repository root, with your worktree's `CARGO_TARGET_DIR` set:

```sh
cargo run -p ergo-sandbox --bin ergo-es -- incident \
  5371373d346aade57f684ead23f386e3441ffb2cb7a860babe96e3c5048b2725 \
  --explorer https://api.ergoplatform.com --out incident-use --network mainnet
```

The explorer feature is enabled by default. The URL is required explicitly;
no explorer is selected implicitly. The output directory must be new, with an
existing parent. Omitting `--out` selects `incident-<txid>`.

Each distinct script spent by the transaction gets `input-N/contract.test.json`,
where N is its first input position. Repeated scripts get one case per input,
with the original `selfIndex`. USE therefore produces three deployed suites:
decoy, swap and pool. `README.md` and `triage.json` are authoring skeletons.
A proposed fixed-source suite must be added by the author; the tool cannot
invent a fix or decide which of the spent scripts is relevant to an incident.

Every case has `expect: "REPLACE-ME"`, deliberately rejected by `ergo-es test`.
Write each expectation from the contract specification. The spending height
comes only from transaction `inclusionHeight`; if absent it is another
`REPLACE-ME`, explicitly flagged in the README. Neither chain tip nor box
creation height substitutes for it. Triage fields are empty; the author supplies
roles, a fresh static finding anchor, authorized releases and search bounds.

The live adapter reads `/api/v1/transactions/<txid>` through
`ChainSource::transaction`. Incomplete input or data-input references are
hydrated using `/api/v1/boxes/<boxId>`. Both paths preserve original hex case.
Only GET requests occur. The live path was **unverified** in the implementation
session; no explorer access occurred in tests. This is the roadmap's
`explorer-dependency` boundary: the fixture path shipped, with no live claim.

The exact scenario projection retains value, ergoTree, boxId, creationHeight,
token ids/amounts in order, and registers as serialized hex in `type: "raw"`
wrappers. Empty tokens/registers are omitted, matching the scenario convention.
Address, box settlement/inclusion height, transaction id/output index, explorer
rendered register values, proofs, headers and preheader are outside this
projection. The scaffold makes no evaluation or exploitability claim; later
suite reductions remain synthetic with `nodeValidated: false`. Nothing is
broadcast.

The USE reproduction test constructs a `Fixture` archive directly from the
three committed USE suites, then round-trips that archive through JSON and the
same scaffold used by the CLI. Those suites contain no boxIds; their inputs
also omit creationHeight, while every output records creationHeight 1868202.
Only missing boxIds and missing input creationHeights get explicit synthetic
fixture values. The equality projection removes precisely those additions;
it retains the committed output creationHeights. Serialized projected inputs
and outputs equal the committed JSON bytes after common JSON serialization,
including integer precision, all hex characters, field sets and array order.
The committed suites are never changed. Separate cases exercise data inputs,
repeated scripts, mixed hex case, missing inclusion height and CLI rejection
of every placeholder.
