# P04: one owned funding proof over canonical transaction bytes

`evidence::sign::sign_owned_p2pk(&request, funding_input, &key)` signs one explicitly
selected standard P2PK funding input and returns P03's `AcceptedExecution` only
after the pinned node's full transaction validator accepts the resulting bytes.
This is a library API, with no browser key storage, HTTP key input, broadcast,
wallet discovery or search integration.

Construct `OwnedDlogSecret` from an explicitly supplied `Zeroizing<[u8; 32]>`.
It rejects zero/out-of-range scalars, has redacted Debug output, and has no serde
implementation. Owned scalar storage is zeroized on drop; this does not promise
that callers' original copies or all temporary machine/register copies are erased.
The key is a separate argument, never a field of the request or report. Caller
metadata is preserved, not sanitized: callers must not put secrets into a case.
Legacy scenario `SecretSpec` remains unchanged and is not used by this API.

The selected input must resolve to a canonical box in the supplied UTXO snapshot.
Its exact script bytes must equal the node's standard P2PK encoding for the owned
key's public image. A role label or a declared public key does not satisfy that
check or generate a proof. Nonstandard DLog scripts, additional newly signed
inputs and new sigma protocols are outside this signing operation. The operation
preserves all other input proofs and extensions; it supplies no missing companion
proofs. Any imported proofs are still checked by the full validator.

The signing message is the node's `bytes_to_sign` for the complete canonical
transaction, including context extensions. The node wallet's existing
`prove_sigma` produces the DLog proof. Inserting it must preserve the signing
message and transaction ID. The signed bytes go through P03's full raw-byte
pipeline, using the original explicit UTXOs, context, parameters, rules and cost.
Proof generation never substitutes for transaction acceptance.

When a P02 wire attachment exists, its bytes must match the request before signing.
The new case retains that original attachment and its provenance under
`ownedFundingProof.previousWireTransaction`, along with the parent request
fingerprint. Its current wire attachment carries the signed bytes. Box origins,
source material and deployment identity remain unchanged. The signing annotation
is descriptive metadata, not trusted proof of ownership on import: fresh node
validation checks the actual signature.

Replay an imported signed `ValidationRequest` directly through `validate`, without
a key. Signing refuses to overwrite an existing funding proof. Imported accepted
report flags remain untrusted; deserialize its request and validate again.
Acceptance applies only to supplied state/rules. It establishes neither historical
membership nor a violated security property. Hypothetical inputs remain hypothetical.

The P04 [manifest](../ergo-sandbox/tests/fixtures/evidence/proof-manifest.json) pins
one public authored experiment with unsigned and signed requests: a keyless input
and one owned P2PK input, carrying a nonempty context extension. Its test key is
public and deliberately hypothetical. Never fund it. Tests verify every artifact
hash and node revision before using it. Recorded signed bytes replay without
constructing any key; fresh proofs use the node wallet's operating-system RNG
and need not equal the recorded proof bytes.

The transaction-change test independently changes outputs, an extension, and
input order. Each old proof fails at the node's proof-verification stage, while a
fresh proof accepts the changed transaction. Missing context and future-output
controls prove that the signing API requires full validation. Two compile-fail
doctests prohibit key serialization/deserialization; replay/error checks verify
that generated bundles do not carry the scalar or secret fields.
