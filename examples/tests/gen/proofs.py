"""Suites for the proof-bearing protocols: the registry (AVL+ proofs from
the sandbox's prover) and the mixer (Schnorr and Diffie-Hellman proofs from
the sandbox's secrets). Run from the repo root after `cargo build -p
ergo-sandbox`; uses `ergo-es point` and `ergo-es compile`."""
import json, os, subprocess, hashlib
E = os.path.expanduser("~/.cache/cargo-target/debug/ergo-es")
def point(secret, base=None):
    args = [E, "point", secret] + (["--base", base] if base else [])
    return subprocess.check_output(args, text=True).strip()
def tree_hex(path, params):
    p = "/tmp/claude-1000/-home-rkadias-coding-ergo-forge/22e78c68-148e-4bbb-a0c8-da7693519c7e/scratchpad/gen_params.json"
    json.dump(params, open(p, "w"))
    out = subprocess.check_output([E, "compile", path, "--params", p], text=True)
    return [l.split()[1] for l in out.splitlines() if l.startswith("tree:")][0]
X = lambda n: "%064x" % n
A = "028333f9f7454f8d5ff73dbac9833767ed6fc3a86cf0a73df946b32ea9927d9197"
p2pk = lambda k: "0008cd" + k
sp = lambda v: {"type": "SigmaProp", "value": v}
ge = lambda v: {"type": "GroupElement", "value": v}
cb = lambda v: {"type": "Coll[Byte]", "value": v}
def write(name, path, params, cases):
    json.dump({"source": open(path).read(), "params": params, "scenarios": cases}, open(f"examples/tests/{name}.test.json", "w"), indent=2)
    print("wrote", name, len(cases))

# ── registry ───────────────────────────────────────────────────────────
alice_name = hashlib.blake2b(b"alice", digest_size=32).hexdigest()
bob_name = hashlib.blake2b(b"bob", digest_size=32).hexdigest()
carol_name = hashlib.blake2b(b"carol", digest_size=32).hexdigest()
NFT = "aa" * 32
def reg_case(name, expect, ops, out_tree="@avl.names.after", fee=10**8, outputs=True, vars=None, secrets=None, residual=None):
    c = {"name": name, "expect": expect, "height": 1,
         "avl": {"names": {"keyLength": 32, "entries": [[alice_name, "01"], [bob_name, "02"]], "operations": ops}},
         "selfBox": {"value": 10**9, "tokens": [{"id": NFT, "amount": 1}], "registers": {"R4": {"type": "AvlTree", "value": "@avl.names"}}},
         "contextVars": vars if vars is not None else {"0": cb("@avl.names.proof"), "1": cb(carol_name), "2": cb("03")}}
    if outputs:
        c["outputs"] = [{"value": 10**9, "ergoTree": "$self", "tokens": [{"id": NFT, "amount": 1}], "registers": {"R4": {"type": "AvlTree", "value": out_tree}}},
                        {"value": fee, "ergoTree": p2pk(A)}]
    if secrets: c["secrets"] = secrets
    if residual: c["expectResidual"] = residual
    return c
insert_carol = [{"insert": {"key": carol_name, "value": "03"}}]
write("registry", "examples/contracts/protocols/registry/registry.es", {"registrar": sp(A), "fee": {"type": "Long", "value": 10**8}}, [
    reg_case("register a new name with the prover's proof and the new digest, paying the fee: no key needed", "pass", insert_carol),
    reg_case("the fee one nanoERG short: only the registrar could sign that", "needsProof", insert_carol, fee=10**8 - 1, residual=A[:8]),
    reg_case("successor keeps the OLD digest: refused", "needsProof", insert_carol, out_tree="@avl.names", residual=A[:8]),
    reg_case("a proof for a lookup does not authenticate an insert", "needsProof", [{"lookup": {"key": alice_name}}], residual=A[:8]),
    reg_case("registering a name that already exists: the tree refuses the proof", "needsProof", [{"lookup": {"key": alice_name}}],
             vars={"0": cb("@avl.names.proof"), "1": cb(alice_name), "2": cb("03")}, out_tree="@avl.names", residual=A[:8]),
    {"name": "the registrar may spend freely (an upgrade), proving with the registrar's key", "expect": "needsProof", "expectResidual": A[:8], "height": 1,
     "avl": {"names": {"keyLength": 32, "entries": [[alice_name, "01"]], "operations": []}},
     "selfBox": {"value": 10**9, "tokens": [{"id": NFT, "amount": 1}], "registers": {"R4": {"type": "AvlTree", "value": "@avl.names"}}}},
])

# ── mixer ──────────────────────────────────────────────────────────────
x, y, z = X(0x1a11ce), X(0xb0b), X(0xeee)   # Alice's secret, Bob's secret, a stranger's
g = point(X(1)); u = point(x)
c1 = point(y)              # g^y
c2 = point(y, base=u)      # u^y
full_params = {"u": ge(u)}
full_tree = tree_hex("examples/contracts/protocols/mixer/full-mix.es", full_params)
def full_case(name, expect, r4, r5, secrets, residual=None):
    c = {"name": name, "expect": expect, "height": 1, "selfBox": {"value": 10**9, "registers": {"R4": ge(r4), "R5": ge(r5)}}, "secrets": secrets}
    if residual: c["expectResidual"] = residual
    return c
alice_dht = lambda h: [{"dht": {"g": g, "h": h, "x": x}}]   # proves u = g^x and h^x
write("mixer-full", "examples/contracts/protocols/mixer/full-mix.es", full_params, [
    full_case("box (g^y, u^y): the owner proves the Diffie-Hellman tuple with x", "proofAccepted", c1, c2, alice_dht(c1)),
    full_case("box (g^y, u^y): the spender cannot (c2 = u^y is not his discrete log)", "needsProof", c1, c2, [{"dlog": y}]),
    full_case("box (u^y, g^y): the spender proves the discrete log of c2 with y", "proofAccepted", c2, c1, [{"dlog": y}]),
    full_case("box (u^y, g^y): the owner cannot (c2 = g^y is not c1^x)", "needsProof", c2, c1, alice_dht(c2)),
    full_case("a stranger cannot spend either box", "needsProof", c1, c2, [{"dlog": z}]),
])
half_params = {"fullMix": cb(full_tree)}
def half_case(name, expect, outputs, secrets, residual=None):
    c = {"name": name, "expect": expect, "height": 1, "selfBox": {"value": 10**9, "registers": {"R4": ge(u)}}, "secrets": secrets}
    if outputs is not None: c["outputs"] = outputs
    if residual: c["expectResidual"] = residual
    return c
mix_out = lambda a, b, value=10**9, tree=full_tree: {"value": value, "ergoTree": tree, "registers": {"R4": ge(a), "R5": ge(b)}}
bob = [{"dht": {"g": g, "h": u, "x": y}}]   # proves c1 = g^y and c2 = u^y
write("mixer-half", "examples/contracts/protocols/mixer/half-mix.es", half_params, [
    half_case("the owner takes the funds back", "proofAccepted", None, [{"dlog": x}]),
    half_case("a partner mixes: two full-mix boxes (c1,c2) and (c2,c1), proving c1 = g^y and c2 = u^y", "proofAccepted", [mix_out(c1, c2), mix_out(c2, c1)], bob),
    half_case("a partner who cannot prove the tuple (wrong secret) is refused", "needsProof", [mix_out(c1, c2), mix_out(c2, c1)], [{"dht": {"g": g, "h": u, "x": z}}], residual=u[:8]),
    half_case("the mirror box not mirrored: refused", "needsProof", [mix_out(c1, c2), mix_out(c1, c2)], bob, residual=u[:8]),
    half_case("a full-mix box with less value: refused", "needsProof", [mix_out(c1, c2, value=10**9 - 1), mix_out(c2, c1)], bob, residual=u[:8]),
    half_case("outputs under another script: refused", "needsProof", [mix_out(c1, c2, tree="10010101d17300"), mix_out(c2, c1, tree="10010101d17300")], bob, residual=u[:8]),
    half_case("three outputs: only the owner", "needsProof", [mix_out(c1, c2), mix_out(c2, c1), mix_out(c2, c1)], bob, residual=u[:8]),
])
