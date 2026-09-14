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
selection and is not pinned to a new tag. The lockfile step first probes
`ergo-es --help` for `verify-lock`; a binary that predates it (every release
before this batch, since this batch publishes none) fails the step with a
compatibility error naming `version: source` as the fix, instead of failing
on an unknown subcommand.

Every verify/lock result carries `identity::LIMITATION`. Structural identity
under constant substitution does not establish behavioural equivalence, safety,
deployment or compiler/source provenance. Static findings only set review
priority. Box observations and structural verification are not node validation.

## Watch: source observations under protocol NFTs (I03)

```sh
ergo-es watch contract.lock.json --nft <tokenId> --register R4 R5 --json
ergo-es watch contract.lock.json --nft <tokenId> --register R4 \
  --baseline box.json --explorer https://your-explorer.example --json
```

`ergo_sandbox::watch::observe(&[WatchInput], Option<&dyn ChainSource>)` returns
one `WatchReport` per lockfile/NFT pair. The CLI applies every repeated `--nft`
to every supplied lockfile; the HTTP/library shape allows different NFT lists
for each lockfile. It accepts at most 64 pairs and unique register names R4–R9
per lockfile. Malformed inputs and excess pairs fail before any source call.
Watch compares the supplied lock's `treeHex`; it does not recompile or establish
that the lock's source or metadata is authentic. Use `verify-lock` for that
independent compilation comparison.

Each report includes `nft`, `lockfileFingerprint`, `chainSource` (`kind`, `url`),
`height`, `live` (the unchanged I02 `LiveComparison`), `registers`,
`baselineBox`, `baselineOrigin`, the full identity `limitation`, and an explicit
`observation` statement. The fingerprint is SHA-256 of compact schema-1 Lockfile
JSON emitted by serde: schema field order, parameter names sorted, no trailing
newline. Input JSON whitespace and object key order do not affect it.
`height` is the source height read immediately before that NFT lookup; it is
neither the holder's creation height nor an atomic chain snapshot. `live.boxId`
identifies the holder returned by the source. Chain membership and currentness
are **not independently verified**.

The existing `lockfile::compare_live` checks singleton metadata and one unspent
holder page (offset 0, limit 2), then delegates to `compare_live_box`. Watch
captures that same response for registers; it never fetches a second version
of the box. Zero or multiple holders, inconsistent token data, incomplete
responses, missing height and source failures remain `unverified`. Transport
retries remain those of the existing explorer adapter; there is no pagination
or background polling. Without an explorer the exact reason is
`no explorer configured (explorer-dependency)`, including in HTTP 200 reports.
An explicit CLI `--explorer` or `EXPLORER_URL` can configure the source; there
is no default public URL. Featureless builds retain the offline path and report
an explicitly configured but unavailable explorer adapter as `unverified`.

Each watched register reports `current`, `expected`, `status` and `reason`.
Values are serialized constant hex as supplied, compared as bytes (hex case is
irrelevant), with no interpretation of upgrade authority or behavior. A missing
or invalid hex value on either side is `unverified`: the ChainBox format cannot
distinguish an absent register from one omitted by the source. Otherwise the
status is `unchanged` or `changed`. With no supplied baseline, the first complete
observation establishes it and explicitly says that no earlier value was
supplied. `unchanged` on that first observation asserts no history.

The response's `baselineBox` can be saved and supplied on the next invocation.
A first baseline is returned only when all watched registers have readable
values. A supplied baseline is retained even across failed observations. CLI
`--baseline box.json` uses that supplied box as the register reference for every
requested pair. It accepts the `ChainBox` shape: `boxId`, `ergoTree`, `value`,
optional `tokens` (`id`, `amount`), and `additionalRegisters` (raw hex strings or
objects with `serializedValue`; `registers` is an alias). Baselines are caller
references; Watch does not attest their chain or NFT membership.

| CLI exit | Meaning |
|---|---|
| 0 | Every script is `bytes_match`; all watched registers are `unchanged` |
| 3 | At least one script is `bytes_differ` or register is `changed` |
| 4 | Any script or watched register is `unverified`; takes precedence over 3 |
| 1 | Input error; no complete report batch |

`POST /api/v1/watch` accepts inline lockfiles with per-lockfile inputs:

```json
{
  "watches": [{
    "lockfile": {"...": "complete schema-1 lock object"},
    "nfts": ["<64-hex-token-id>"],
    "registers": ["R4"],
    "baselineBoxes": {"<64-hex-token-id>": {"...": "supplied ChainBox"}}
  }]
}
```

`registers` and `baselineBoxes` are optional. The response is an array of reports.
The route only uses `state.cfg.explorer_url`; request URLs and fixture overrides
are rejected. It shares the request body cap, rate limiter and engine budget.
No request or baseline is stored server-side. The Read **Watch** section accepts
a pasted or uploaded lockfile, NFT IDs, register names and an optional baseline
box. It retains first-observation baselines in page memory for repeated clicks;
editing inputs or pressing **Reset baseline** clears them. Reloading the page
clears them too. Watch is disabled with an explanation when no explorer is
configured. The lockfile defines Watch's comparison independently of the tree
currently displayed by Read.

Watch constructs no transaction, signs nothing and broadcasts nothing. Tests
use fixtures and in-process HTTP requests with no network. The recording fixture
asserts the exact bounded reads; source checks guard the module, CLI and route
against signing/broadcast dependencies. Static findings remain review priorities;
Watch reports only the supplied script and register byte observations.
