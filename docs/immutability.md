# Deployment verification and compilation lockfiles

`ergo-es verify <address|treeHex> --source contract.es --params params.json
--network mainnet --json` compiles on the pinned engine, decodes addresses
offline, and delegates to `identity::match_trees`. A box caller supplies its
`ergoTree` bytes. Hex always means tree bytes, even at 32 bytes; no box lookup
or explorer is involved. Address checksums and the selected network are checked.

The JSON `outcome` enum and CLI exits are:

| Outcome | Exit | Meaning |
|---|---:|---|
| `exact` | 0 | Identical serialized ErgoTree bytes |
| `template` | 3 | Identity reports the same program with differing constants |
| `no_match` | 4 | Neither positive condition holds |
| Input/engine error | 1 | No comparison result |

Template results include **every** `identity::ConstantDifference`, unchanged,
in `constantDifferences`. `left` is compiled source, `right` is the supplied
target. Each occurrence has its ordered child `path` and complete typed values,
including constant-table indices (null for inline values). No similarity score
is exposed. A structural near miss is `no_match`. Trees that differ only in
serialization flags or unused constants, with no substituted constants, are
also `no_match`: they do not meet either positive condition.

Write's **Verify deployment** control sends the current source, typed params,
network and target to `POST /api/v1/verify`. The route takes the compile request
shape (`source`, `params`, `network`, `treeVersion`) plus `target`. It shares
compile's body limit, engine budget, rate limiting and large-stack worker.
Changed source, parameters, network or target invalidate displayed verification.
Failures and stale responses cannot infer a match.

## Lock schema 1

```sh
ergo-es lock contract.es --params params.json --network mainnet --out contract.lock.json
ergo-es verify-lock contract.lock.json --source contract.es --params params.json --network mainnet --json
```

The JSON object contains `schemaVersion`, `sourceSha256`, `params`,
`compilerRevision`, `nodeRevision`, `workbenchVersion`, `treeHex`, `address`,
`treeVersion`, `lintSweepDigest`, and `limitation`. Unknown fields are rejected.

- `sourceSha256` is SHA-256 of the exact UTF-8 source text: whitespace, comments,
  line endings and a final newline all count. No trimming or normalization.
- `params` retains the supplied typed JSON map, sorted by parameter name. It
  records supplied values, including unused values; template defaults remain
  bound by the source and engine. Verification uses the independent supplied
  params file (empty map if omitted), rather than trusting locked parameters.
- Both revision fields reuse `evidence::case::engine_revision()`, which reads
  the pinned compiler revision from the workspace manifest. All node crates
  in this batch share `9468043396e5daa2828211bcff4234bc70fae4f0`.
- `treeHex` is lowercase serialized ErgoTree hex. `address` is the P2S address
  for the selected network. `treeVersion` is the actual emitted tree header
  version, which can be 0 even when the compiler option is 3.
- `lintSweepDigest` is SHA-256 of compact UTF-8 JSON containing arrays
  `[lint_id, node_id, message]` for **all** `audit::audit` findings on the tree.
  Sort by lint id lexicographically, node id numerically, then message
  lexicographically; retain duplicates. No trailing newline. Empty findings
  hash `[]`. Lift uses mainnet rendering on every network to keep the digest
  a function of the tree. Severity and provenance are outside this projection.
  This records the current static lint sweep for this tree, not a hash of the
  historical S02 corpus report and not a vulnerability assertion.

Both commands accept `--tree-version N` (compiler option, default 3) and
`--network mainnet|testnet` (default mainnet). Supply the same independent
options when verifying a nondefault build. Inferring these options from the
lock's address or tree version would conceal drift in the field being checked.
`POST /api/v1/lock` accepts the compile request shape under the same engine and
request limits and returns the lock object.

`verify-lock` recompiles and compares each recorded field independently. JSON
`drifts` includes `kind`, `field`, `locked` and `current` for each change. Kinds
are `schema_version`, `source`, `params`, `compiler_revision`, `node_revision`,
`workbench_version`, `tree_bytes`, `address`, `tree_version`, `lint_digest`,
and `limitation`. A changed input can cause multiple real derived drifts;
changing one recorded field reports only that field. Compilation failure is an
error, never a current lock. Local status is `current` or `drift`.

## Optional observed box comparison

```sh
ergo-es verify-lock contract.lock.json --source contract.es --params params.json \
  --explorer https://your-explorer.example --nft <tokenId> --json
```

`EXPLORER_URL` can supply the explorer URL, as in the web configuration. There
is no default public explorer. Only an explicit `--nft` triggers lookup. The
existing `ChainSource` supplies singleton token metadata and a bounded page
(limit 2) of unspent holders. Missing data, failed transport, ambiguous holders,
a non-singleton token or a box that does not hold one unit remain `unverified`.
The comparison checks **bytes**, through the same `compare_live_box` function
for fixture and explorer responses. `live.status` is `bytes_match`,
`bytes_differ` or `unverified`; local status stays independent.

Without an explorer, live status is always `unverified` (`explorer-dependency`).
No chain access runs in tests. The real USE explorer transaction recording in
`fixtures/evidence/claim-vectors/use-archive.fixture` supplies the historical
box for the deterministic comparison. A typed SigmaProp parameter containing the
recorded public key makes the pinned compiler reproduce its P2PK bytes exactly;
the positive control uses a freshly generated lock without replacing any field.
This byte equality does not assert historical source provenance. Recorded metadata for the fixture lookup
is a test input, not a fresh singleton/currentness attestation.

CLI exit 0 means local fields are current and any requested live comparison
returned equal bytes. Exit 5 means local drift or differing observed box bytes;
6 means a requested live check is unverified; 1 means an input/build error.
When no live check is requested, exit 0 can accompany `live.status: unverified`.
Explorer observations do not independently verify chain membership or freshness.
Builds without the `explorer` feature retain the fixture and unverified paths.

## Project zips and CI

Project exports from Write and Build include `contract.es`, `params.json`,
`contract.test.json`, `contract.lock.json`, and a README with verification
commands. The source/params/network snapshot goes to `/api/v1/lock` before the
zip is created; failed compilation prevents an incomplete export. The existing
STORE-only zip writer is unchanged. Exporting does not assert tests have run.

The `.github/actions/test` composite action accepts optional `lockfile`, a
space-separated list of paths/globs (quote a pattern containing spaces).
For each `<stem>.lock.json`, it checks sibling `<stem>.es` and optional
`params.json`. Sibling `<stem>.test.json` supplies independent `network` and
`treeVersion` options, defaulting to mainnet and 3. This is the exported project
layout. Custom names must follow that convention. Unmatched patterns, missing
source, malformed options and any nonzero verifier exit fail the action.
Suites still run as before; `version: latest` keeps GitHub's latest-release
selection and is not pinned to a new tag. The selected binary must include the
new `verify-lock` command; this batch does not publish a release.

Every verify/lock result carries `identity::LIMITATION`. Structural identity
under constant substitution does not establish behavioural equivalence, safety,
deployment or compiler/source provenance. Static findings only set review
priority. Box observations and structural verification are not node validation.
