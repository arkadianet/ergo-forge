# USE recovery: source-recorded context and native derivations

Option B recovered the missing material without lowering P05's gate. The
[governing decision](../P05-DECISION.md) preserves the original stop as history.
These artifacts support a reproducible transaction/property claim under recorded
premises; they do not authenticate the canonical chain or historical UTXO set.

[retrievals.json](retrievals.json) records every request URL, UTC retrieval time,
HTTP status, raw response path and SHA-256. Responses are preserved byte-for-byte.
The earlier incident response and its retrieval source remain in
[the original stop evidence](../p05-stop-evidence/sources.json). `fetch.py` is the
one-time raw-response recorder, accepting an explicit JSON array of `[name, URL]`
pairs. Neither the acceptance test nor the replay command invokes it. Refetching
is not a gate repair: explorer confirmation counts can change response hashes.
The public node's [API schema](https://node.ergo.watch/swagger) supplied the
block-at-height and header-slice routes.

The map's `ExplorerSource` and `RecordingSource` were inspected, not expanded.
[ChainBox](../../ergo-sandbox/src/map/source.rs#L44) deliberately omits complete
creation references and proof extensions; its source trait also has no historical
parameter/header snapshot operation. Wrapping that projection in a recorder
would preserve its omissions. Raw node and explorer responses are recorded here
without changing the frozen map abstraction.

Every derived request is constructed by
[recover.rs](derive/src/recover.rs#L74), which is shared with the P05 integration
test. It reads files, hashes them, uses pinned node codecs and validates native
transactions; it neither fetches nor writes. The optional producer in
[main.rs](derive/src/main.rs) writes derivative request files. The gate does not
trust those generated requests or previously accepted flags: it reconstructs the
USE request from sources and compares every field to the admitted bundle.

| Material | Recorded input and derivation | Executed check |
|---|---|---|
| Incident identity/order | `node-block.fixture` header and ordered `blockTransactions.transactions`; original public incident response | Incident is transaction index 5; IDs and block ID agree. Every reconstructed transaction ID uses node signing bytes. |
| Spending boxes | Original incident response and `prior-0.fixture` through `prior-4.fixture`: value, ErgoTree, ordered assets, serialized registers, `outputCreatedAt`, creating transaction ID and output index | Node canonical serialization reproduces each full box ID. The initial builder is only a codec construction step; `WireBox::recorded` is created only after equality with the archived identity. |
| Spending proofs/extensions | Node block's `inputs[].spendingProof` | Explicit proof bytes and every typed extension constant are parsed with the node codec, included in canonical transactions, and validated. No empty-extension default is used. |
| Header chain | `epoch-chain.fixture`, epoch header 1867776 through incident header 1868204 | All 429 native serialized header IDs match; parent links and heights are consecutive; endpoints match the epoch and incident block. Header v4's empty unparsed suffix is accepted only because the serialized ID matches. |
| Block commitment | Node block's transaction IDs and proof witnesses; `header.transactionsRoot` | The pinned node Merkle implementation checks the root. Witness derivation follows pinned `ergo-validation/src/block/validate.rs:160` using the node hash primitive. IDs for the six executed transactions are additionally recomputed from full canonical transactions. This is not full validation of every transaction in the block. |
| Epoch selection | Incident height 1868204 and pinned mainnet voting length 1024 identify boundary 1867776; `epoch-ids.fixture` resolves it | The linked source chain starts at that recorded boundary and terminates at the incident; its extension is parsed by `parse_active_params`. |
| Active voted parameters | `epoch-block.fixture` extension fields, namespace `00` | Native extension commitment equals the header root; `parse_active_params` and `ProtocolParams::from_active` supply the values. Current `/info` parameters are not substituted for epoch evidence. |
| Fixed protocol constants | Pinned `ergo-validation/src/context.rs`, `ProtocolParams::from_active` | The node supplies max box size, token limit and storage period; these are not reconstructed as independently chosen forge defaults. |
| Preheader context | Incident header height, miner key, timestamp, version, parent, nBits, votes; epoch active block version | Fields are copied/decoded explicitly. Activated script version is active block version minus one, matching the pinned node's version convention. No P03 hypothetical context is reused. |
| Context headers | Last ten predecessors from the linked source chain | Native serialized bytes, newest first, excluding the incident header. `previous-headers.fixture` records the supporting public slice as well. |
| Network rules | Pinned `ergo-chain-spec::ChainSpec::mainnet`, `ReemissionParams::mainnet` and `emission_script_trees` | Explicit EIP-27 activation, token ID and native pay-to-reemission tree are supplied. `/info` records the source node's mainnet identification; the gate does not infer a network from absent fields. This remains explicit network configuration, not an authenticated genesis-to-tip proof. |
| Prior block cost | Fresh validation of transaction indices 0–4 under the same recovered snapshot | Start at zero only at block index 0; carry each native `totalBlockCost` into the next request. USE receives 194317, not zero; its cumulative result is 216345. |
| Local admission policy | Explicit caller-supplied max transaction size equal to the recorded epoch block-size limit | This is the replay's local policy, not a claim about the historical node operator's settings. It is fingerprinted and labelled `caller-supplied`. |
| Roles and extraction objective | P05 USE property, separate from execution sources | The declared attacker/companion/protected roles and existing versioned accounting are retained. The tool does not infer ownership or protocol intent from the source archive. |

Native derivations are pinned to node revision
`9468043396e5daa2828211bcff4234bc70fae4f0`. The extra test-only `ergo-chain-spec`
and `ergo-crypto` dependencies use that same revision; no new engine or validator
version was introduced. The standalone recovery crate has its own lockfile;
the acceptance helper runs against the workspace lockfile.

The previously borrowed parameter snapshot used input cost 2000, output cost 100,
and block size 524288. The recovered epoch values are 2407, 298, and 1271009.
Those are newly sourced context values, not edits to any baseline, corpus row,
search cap, or previous fixture. The old rejected candidate remains unchanged.

To rerun the acceptance derivation without fetching or rewriting anything:

```sh
CARGO_TARGET_DIR=./target-p00 cargo test --release -p ergo-sandbox --test claim_replay -- --nocapture
```

The target requires the old hypothetical candidate to fail the public-source
comparison. Substituting its parameters, rules, block context, headers or prior
cost while changing only the label to `source-recorded` must also fail.

To reproduce derivative request files explicitly (a fixture-author operation,
not an acceptance step):

```sh
CARGO_TARGET_DIR=./target-p00 cargo run --locked --release --manifest-path docs/p05-recovery/derive/Cargo.toml
```

Adding `-- --write-claim` explicitly rewrites only the recovered USE bundle and
its manifest hashes. It preserves the existing authored sale files and controls.
An ordinary third party can replay the complete saved bundle without this crate,
without the source directory and without network access. Share this directory as
well when the third party needs to inspect and rerun the provenance derivation.
