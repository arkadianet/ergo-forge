# D00 governing decision — 2026-09-11

The author authorizes incremental registration: append D00 only, retaining the
full proposed D00–D04 queue in discovery design section 5.3. CI cannot enforce
gates for unregistered units. That is the explicit cost of this scheduling
decision; CI acceptance semantics remain unchanged. Later registration requires
another governing decision. Capability 1 may be the endpoint.

Preserve M00’s original manifest bytes and protectedPolicySha256
`41729240759244bd3edc833e31359bfe50d8430f0b5a2e27ebc727706363cd10`.
Project only the original P-era units and original resolution entries for that
anchor. Separately authenticate the original policy snapshot: every original
P/M registration and P05 resolution must be identical; only the exact D00
registration (implementation false before completion, true after passing),
completedThrough M04 or D00 respectively, and the exact D00 resolution may be
added. Reject changes to every other protected field, extra units, changed,
missing, duplicated or reordered registrations/resolutions. The new snapshot
and permitted resolution are independently byte-pinned by the M00 harness.
The M00 anchor is never re-derived from the amended policy.

Resolve the historical D00 stop using the existing policy-pinned mechanism.
Retain all original evidence and record fields, adding only resolution metadata.
The first blocker was the author’s contradictory scheduling instruction; the
second was a real latent defect in M00’s projection. This decision removes those
registration blockers; it grants neither D00 completion nor semantic authority.

Implement the original 24-row synthetic D00 inventory and separate transfer
registration before an evaluator exists. Validate reference transactions
individually via the current full node API; these calls do not check linked
traces. Record extraction output as measured. No independent human review has
been supplied: semantic measurement remains incomplete. Missing defensible
transfer references leave its denominator null and cancel capabilities 2 and 3
at the checkpoint under the unchanged design. Do not fabricate review or utility.

No baseline, corpus member, cap, verdict, denominator, expected fact or CI
acceptance rule changes. The tool refutes and never establishes; holds-on-execution
is not proved or safe. Refuting a mistaken author assertion is not necessarily
a contract defect. No later implementation, Git mutation, network acquisition,
protocol contact, publication or broadcast is authorized here.
