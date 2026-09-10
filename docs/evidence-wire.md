# P02: node wire bytes, still unvalidated

`ergo_sandbox::evidence::wire` constructs and imports boxes and transactions using
only the pinned node's `ergo-ser` codecs. It does not call a transaction validator,
create or verify proofs, fetch state, or change legacy scenario execution.

`CandidateSpec` requires value, ErgoTree hex, creation height, ordered tokens,
and the full node encoding of dense additional registers (count byte included;
`00` means explicitly empty). Missing fields, invalid hex, malformed or trailing
material are errors. The node's `try_from_raw_parts` checks parsed/raw consistency;
there is no trusted-parts shortcut and no replacement tree. Node-supported opaque
ErgoTrees remain opaque bytes, not evidence that a script can execute.

`WireBox::hypothetical(spec, reference)` requires an explicit creation transaction
ID and output index. It creates a **new hypothetical** box and computes its ID
from full serialization. Even an all-zero reference must be supplied explicitly.
`WireBox::recorded(material, locator, revision)` instead requires complete box
bytes, the tree bytes needed by the node's standalone parser, and the claimed box
ID. It rejects an ID mismatch rather than repairing it. Parsing retains the
creation reference embedded in those bytes. `from_record` rechecks imported
provenance and wire material; recording a source establishes no UTXO membership.

`WireTransaction::build` accepts node inputs (including proof and extension),
node data-input references and candidate specs. `from_node` also checks that the
node's serialized bytes agree with its parsed fields, rejecting stale raw caches.
`from_bytes` requires the claimed transaction ID and rejects mismatches, trailing
bytes and truncation. Imports require exact node write-back equality: they never
normalize an observed record into a different byte sequence. Raw forms retained
by the node's own codecs remain retained; this is not an independent canonicality
or consensus validator.

`bytes_to_sign()` and `id()` delegate to the node's corresponding functions.
The signing message is the **signed wire layout with empty proofs**, preserving
extensions; it is not the node's unsigned-transaction wire layout. Proof changes
therefore change full transaction bytes while preserving signing bytes and ID.
Extension changes affect signing bytes and ID. `output_boxes()` uses the computed
transaction ID and each output's index, marking the resulting boxes hypothetical
because transaction acceptance and historical state have not been established.

`bind_case` associates resolved spending/data-input boxes with the transaction's
references and attaches exact transaction, proof and extension bytes to a P01
case. Box order is spending inputs followed by data inputs, with both counts
recorded. Existing context, target, source and assumptions survive; the previous
missing-state premise is retained in the attachment. Occupied state or wire slots
are rejected instead of overwritten. Case identity includes the complete signed
bytes, so changing only a proof invalidates case reuse even though transaction ID
is unchanged. `deploymentIdentity` remains `unknown`, and `nodeValidated` remains
false. Saving and importing a P01 case does not trust a stored wire result: rebuild
through `WireBox::from_record` and `WireTransaction::from_bytes` before use.

Legacy `ScenarioBox` lacks the creation reference and original field-presence
information. `canonical_box()` refuses promotion, including when `boxId` is
supplied. Supply original full wire material or explicitly construct a new
hypothetical object. `box_build`, Play, txcheck and browser storage keep their
existing simulation/preflight behavior. No automatic conversion or migration is
introduced.

The [wire fixtures](../ergo-sandbox/tests/fixtures/evidence/README.md) pin two full
boxes, one transaction's full/signing bytes and ID, and its resulting output box.
These are authored codec vectors with a dummy proof, **not accepted transaction
bundles**. The implemented [P03 validator](node-validation.md) is a separate
validation step; these P02 codec fixtures do not establish transaction acceptance.
