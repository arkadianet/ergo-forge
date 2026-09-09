Drain reports use `recognized-attacker-receipts-v1`, independently of synthesis.
For each asset, `extracted = min(V, max(0, A - N - S))`. V includes declared
protected and companion spending inputs; N includes realized outside spending
inputs and token supply growth. Data inputs contribute to neither.

Only `10010101d17300` and exact standard P2PK scripts for compressed SEC1 public
keys in `attackerPublicKeys` contribute to A. `attackerTree`, free-payee labels,
and a reducer's `needsProof` do not declare key possession. Protected and
companion inputs must still reduce to `pass`.

Requests must explicitly declare `objective`. For custody with no authorized
releases, use `"objective": {"terms": []}`. Map-derived requests leave this
policy unset for the caller to supply. Missing policy produces
`incompleteObjective`; unsupported JSON fields fail deserialization explicitly.

A static allowance names one declared victim spending input (its index before
permutation), an asset (`nanoErg` or token id), an exact destination script, and
an upper quantity bound:

```json
{"sourceInput": 0, "asset": "nanoErg", "destinationTree": "10010101d17300", "maxAmount": 1000000}
```

Put allowances in `objective.terms`. Template free amounts grant no implicit
allowance. Box value, refunds and fees need their own explicit allowances.
Overlapping allowances with identical source/asset/destination bindings combine
by maximum. A capacity graph maximizes the total authorized receipt subject to
shared source holdings and realized recipient holdings, preventing double counts
and greedy source-allocation errors.

A term may additionally contain `payment` with `asset`, `sellerTree`,
`amountPerUnit`, and `retainedTrees`. These fixed-rate terms authorize the receipt
only when the entire released quantity is paid for. Released quantity is the
source's holdings minus realized holdings at the seller and declared legitimate
continuation/owner scripts. Consideration is the net increase of the payment
asset at the exact seller script. Output position and labels do not bind payment.
An ERG payment sanctions stock only through its explicit stock term.

Conditional support is bounded: distinct terms sharing a seller/payment-asset
pool produce `incompleteObjective`, because joint payment allocation is not
implemented. Exact duplicate terms are deduplicated. Variable rates, arbitrary
predicates and automatic contract-term inference are not supported; declare a
valid bounded static policy or an appropriate fixed-rate term. Corpus bank
policies apply to their fixed declared state; they are not a general bank model.

JSON hits expose `V`, `A`, `N`, `S`, `recognizedOutputIndices`,
`unknownOutputIndices`, `termsUsed` and `extracted` together. Each evaluated term
records its binding, release quantity, net payment, satisfaction and eligible
allowance before shared capacity caps. The report also records policy and key
assumptions. `firstAccounting` retains the first accepted transaction's measurement
including zero-score cases; `best` carries the highest-scoring witness.
`custodyDeficit` and `nftDetached` are separate diagnostics. The text CLI prints
components, recognized/unknown indices and terms alongside extraction.

The phase-3 corpus baseline is in `examples/mutants/answer-key.json`; the previous
objective's measurements remain in `answer-key.custody-v1.json`. This change does
not add search axes or alter roles, caps, or rejection precedence.
