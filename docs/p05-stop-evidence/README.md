# Rejected P05 evidence — not an admitted claim

These artifacts document the `missing-provenance` stop in
[../roadmap-stops.json](../roadmap-stops.json). They are outside the accepted
fixture manifest and mutation corpus. No result in this directory counts toward
a baseline, P05 acceptance floor, or deployed-exploit claim.

- `public-block.fixture` and `public-transaction.fixture` are verbatim public
  explorer responses; URLs and SHA-256 hashes are in `sources.json`.
- `rejected-use-candidate.fixture` is the uncommitted prototype bundle with
  borrowed hypothetical validation context. Retaining it exposes the failed
  premise rather than hiding it. It is not endorsed replay evidence.
- `check_provenance.py` verifies artifact hashes and reports the mismatch;
  `python3 docs/p05-stop-evidence/check_provenance.py` exits **1**.
- `provenance.log` is the actual output of that command. The stop record hashes
  both the command source and its inputs/output.

The retrieved public data can support further reconstruction, but it did not
supply the full active validation snapshot in this attempt. Reopening requires
backed context, parameters, rules, header window and prior block cost, followed by
the unchanged P05 acceptance gates. A hypothetical source label is not a substitute
for that evidence. This directory makes no claim of canonical-chain authentication.
