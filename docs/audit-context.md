# Auditing a transaction contract set

`audit(&lifted)` still returns the original single-contract findings.
`audit_with_contracts(&lifted, &ContractSet)` runs those same detectors and
retains every finding, its severity, message, and AST/IR anchors. It adds an
`Active` or `Discharged(evidence)` status. Both the result and evidence serialize
with Serde; each finding has `status: "active"` or `status: "discharged"` and,
for the latter, an `evidence` object.

The context is an **explicit same-transaction execution premise**. It is not
just a list of related source files. Supply the spending scripts with their
input positions, and any resolved context extensions that must also succeed.
The audited `Lifted` must be the same borrowed object as exactly one member.
Mark `complete` only when all required scripts are accounted for. Empty names,
duplicate execution locations or names, missing input positions, orphaned
extensions, and partial/truncated lifts prevent all discharges. NFT proofs also
require `singleton_tokens`, a token-id-to-evidence map recording singleton
issuance; a token name or a current box's token amount is insufficient.

A `ProtocolMap` supplies box names, scripts, references and token classification,
but **map reachability does not prove execution in a transaction**. Resolve it
to this declared set before calling the API. Never turn `Role::Companion`, an
inferred pairing edge, or a data-input script into a required executing script.
The API does not validate transaction membership, extension bytes, or issuance
against a chain source; those are caller premises. Its conclusion is conditional
on them and must not be presented as safety of every possible protocol spend.

For example, a set can contain:

```rust,ignore
let inputs = [
    InputContract {
        name: "phoenix_v1_hodltoken_bank.es",
        execution: Execution::SpendingInput(0),
        lifted: &bank,
    },
    InputContract {
        name: "phoenix_v1_hodltoken_proxy.es",
        execution: Execution::SpendingInput(1),
        lifted: &proxy,
    },
];
let result = audit_with_contracts(&proxy, &ContractSet {
    inputs: &inputs,
    complete: true,
    singleton_tokens: &singleton_issuance_evidence,
});
```

For this set, the proxy's `OUTPUTS(0)` reserve finding remains in the output,
discharged by the bank's required
`OUTPUTS(0).propositionBytes == SELF.propositionBytes` check. Evidence names the
bank, its execution location, the literal slot, `self-successor`, the identity,
the covered dimension, and the binding's AST/IR ids. Script identity does not
claim to constrain reserves: other lints, including `delegated-reserves`, remain
active. Removing the bank removes the discharge.

The map's existing `tree_refs` reader describes syntactic references, including
optional comparisons. Discharge instead uses `required_bindings`: a bounded
positive predicate analysis that follows value aliases, unions conjunctions and
intersects alternatives. It rejects unused checks, negation and unsupported
constructs as proof. A literal false branch cannot succeed. Exhaustion supplies
no proof. Evidence ids identify a representative equality when the same identity
check occurs in multiple alternatives.

Only identical literal transaction slots match. `SELF` is never silently
substituted for a positional input, and context-variable indices do not match
across contracts. Reserve findings accept required NFT (with singleton evidence),
script-hash or output self-successor checks. Data-provider `trust-assumptions`
findings accept only NFT checks with singleton evidence; code identity alone
does not authenticate register provenance. No other lint is discharged.

The regression fixtures cover Phoenix, Lithos's emission guard with its required
extensions, and the real Dexy LP composition gap. Dexy's pool may govern its
successor, but it does not bind the swap's `INPUTS(0)`: that finding stays HIGH,
including when a decoy precedes the actual pool input. The ordinary corpus sweep
has no transaction membership evidence, so it continues to report local findings
without discharging any of them.
