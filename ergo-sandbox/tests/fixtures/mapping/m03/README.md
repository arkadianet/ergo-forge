# M03 canonical synthetic evidence

The unchanged M00 manifest and answer key remain the corpus authority. This
subdirectory adds 53 full-validator transactions for the M03-relevant vectors
(all families except context_code, context_scope and avl_identity) and 20
supplemental transactions. Each `.fixture` request retains canonical inputs and
explicitly supplied hypothetical validation context, rules and parameters.
The tests reconstruct the authored balance outputs and context from the independent
case definitions and compare the whole request before calling the existing full
validator. No source/discovery inference supplies expected acceptance.

`manifest.json` pins the 53 corpus-derived requests. `supplemental-v2.fixture`
pins five new positive/control pairs and 20 requests. Tests pin both documents,
check every request hash and verify accepted outcomes or `ProofFailed` at input 0.
The separate result artifacts live under `docs/mapping/` to avoid changing the
frozen decompiler corpus scanner's input universe.

`supplemental.fixture` is preserved v1 history. Its construction template left
unused truth/rationale/site annotations attached to the new cases. V2 explicitly
states truth about its own claims and removes the unused inherited annotations.
It retains all ten IDs, supported dispositions, targets, guards, execution
verdicts and all twenty request bytes/hashes. V1's measured results remain at
`docs/mapping/m03-mapping-results-v1.json`. This correction is documented in the
design amendment and M03 report; none of the M00–M02 pins was edited.

Claims use canonical fixed SELF, exact root and revision, one guard, explicit
positive-input-token state, and a fixed validator context before SELF becomes
storage-rent eligible. Accepted omission assessments bind to those exact claims
and execution fingerprints. They are M03 test evidence, not the future M04 action
or witness-import API. These are publishable locally authored hypothetical
transactions; none was sent to a network or live protocol.
