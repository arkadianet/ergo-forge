# P06 box provenance: the incident's own boxes, rebuilt and checked

P06 promotes a generated candidate only if its material can be built, signed and
validated. This directory supplies the material for the public-incident family
without inventing any of it.

[retrievals.json](retrievals.json) records every request URL, UTC retrieval time,
HTTP status, raw response path and SHA-256. Responses are preserved byte-for-byte.
[fetch.py](fetch.py) is the one-time recorder; neither the acceptance test nor any
product code invokes it. Refetching is not a gate repair.

The two protocol boxes the incident spends are **source-recorded**. Each is
rebuilt from the block that created it and admitted only if the reconstruction
hashes to the box id the incident request declares:

| Box | Creating transaction | Index | Recorded in |
|---|---|---:|---|
| `7dc430f6…` (protected, 284,695 ERG) | `18d03837…` | 0 | `block-1868090.fixture` |
| `7a35b75a…` (companion) | `18d03837…` | 1 | `block-1868090.fixture` |

Creation height is not inclusion height: both were created by a transaction
included at height 1868090, which is how the recorded blocks were selected.

The attacker's funding box is **hypothetical and archived as such**. A drain is
funded from a box its author controls, and no key for the historical attacker box
is held here, so the test substitutes a P2PK box under a key it generates,
carrying the token holdings the request already declares. Its premise is
hypothetical; the protocol's boxes are not. A claim built on it says exactly that.

Block context, parameters, network rules, the header window and prior block cost
are the ones [P05 recovered](../p05-recovery/README.md) for height 1868204 — the
same height this candidate spends at — and are reused rather than re-derived.

## What promotion refuses

`bind` compares the generated candidate against this material field by field. It
refuses a label-derived or unbacked input id, holdings the material does not
carry, an unavailable data box, a role mismatch, an unsupported context extension
and an incomplete context. A refusal is a named promotion failure: the candidate
stays an unsigned preflight result and earns no claim.

Output *contents* are compared; output *ids* are not. A generated candidate
carries the search's own deterministic simulation ids, and a canonical output id
derives from the transaction that exists only once the candidate is built, so
requiring equality would reject every candidate by construction. Both ids are
recorded in the promotion mapping so a reader can tell them apart. Input ids stay
strict, because those reference boxes that already exist.
