# P02 wire vectors

`wire-v1.fixture` contains authored, hypothetical codec vectors at the recorded
engine revision. The boxes share candidate material (including a token and R4)
but have different creation transaction references and output indices. The
transaction spends the first box, references the second as a data input, and has
one output. Its proof is deliberately dummy bytes `010203`; its context extension
contains Int 7 at key 0. No transaction-acceptance claim accompanies these bytes.

The complete box bytes/IDs, transaction full bytes/signing bytes/ID, and output
box bytes/ID are pinned. `evidence_wire.rs` reads fixture-count floors directly
from ROADMAP's policy block. Tests never regenerate or overwrite vectors.

`generate_wire_vectors.rs` records the one-time generator, which calls pinned
node APIs directly rather than Forge's new wire wrapper. For an explicitly
authorized fixture change, copy it to a temporary Cargo example and run it with
`cargo run --release -p ergo-sandbox --example <temporary-name>`, capturing stdout
to a temporary file for comparison. Delete the temporary example afterward.
Do not silently regenerate expected values to satisfy a failed gate. This producer
checks adapter parity with the pinned node; it is not independent network or
historical-state evidence. These new codec fixtures do not modify any existing
mutation corpus or measurement baseline.

The `.fixture` suffix is intentional: this is JSON consumed explicitly by the
P02 gate, outside the decompiler harness's recursive `.json` enrollment. Adding
a codec vector must not change the frozen decompiler corpus denominator.
