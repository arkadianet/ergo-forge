# P05 governing decision: B, recover the source evidence

**Decision: preserve P05's original gate and recover the USE snapshot. The strongest
argument against this decision is opportunity cost: an incident-specific recovery
could consume the unit without yielding a usable snapshot, while buildable replay
machinery remains shelved.** This decision did not authorize a fallback to a lower
public-incident floor.

An authored fixture establishes behavior under premises selected by its author.
That is useful for implementation controls and the playground, but is insufficient
as the sole evidence that the claim boundary handles independently produced chain
material. Adding another authored fixture to satisfy a family count would not
answer that question. Portability requires exact material that a third party can
rerun; it does not require pretending that chosen premises are observed history.

Option A would also leave P06's `generated_public_incident_candidate_promotes`
requirement unresolved ([ROADMAP section 3](ROADMAP.md#p06--promote-drain-candidates-through-the-evidence-boundary)).
Unblocking the entire queue that way would require further explicit reductions in
scope. Splitting machinery from evidence is a defensible product change, but it
would ship an authored-case capability while leaving the public-evidence milestone
and public promotion blocked. I decline that scope change while the missing data
can be recovered. Option C would preserve honesty but leave a reachable source
question unanswered.

The recovery succeeded: source-recorded headers link the incident to its epoch;
the pinned node codecs reproduce their IDs; the epoch extension matches the
header commitment; the node's active-parameter parser supplies the voted values;
the node's mainnet chain specification supplies the network rules and protocol
constants. The five preceding transactions, including their actual extensions
and proofs, validate in order. Their cumulative cost supplies the incident's prior
block cost. The P05 gate repeats this derivation and checks the complete incident
request against it, including negative checks against the withdrawn candidate.
See [recovery sources and derivations](p05-recovery/README.md).

The stop at `e251f22` remains valid history. Its original fields and hashed evidence
are retained in `roadmap-stops.json`; an appended resolution identifies this
governing decision. Policy pins the complete resolved record and this decision's
hash. The runner may reopen only that exact record, still executing all P05 tests
and predecessor gates. A resolution label on its own cannot bypass a stop, and
reopening is not itself a passing feature gate. New or changed stops still block.

No numerical acceptance threshold changes: three accepted bundles, two positive
claims, two families, one public incident, one accepted nonviolating control, and
zero benign confirmed violations remain required. The public-incident contribution
is source-backed USE; the second family is the existing authored fixed-rate sale
with positive and negative controls. Every baseline/corpus value remains historical
and unchanged. P06–P08 are not implemented by this decision.

The public claim remains bounded: the saved bundle reproduces a node-accepted
transaction and violation of the declared extraction property under explicit
premises. Recorded sources and cryptographic consistency checks do not authenticate
the canonical chain or prove historical UTXO membership. Caller-declared roles and
policy are not inferred protocol intent. The command does not claim protocol-wide
security, fetch missing material, or broadcast a transaction. Failure to rerun
P05's unchanged gate leaves P05 incomplete; completion advances only after every
predecessor gate reruns green.
