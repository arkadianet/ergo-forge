"""Pin the trees the old compiler produced from the eleven sources the 6.0.7 typer rejects.

    python3 docs/reports/historical-trees/regenerate.py --old-ergo-es PATH

PATH is `ergo-es` built from forge main@211c5b5 (node a203cc02), which pins both the
compiler and the engine that produced every recorded tree. This writes two inputs:

  ergo-sandbox/tests/fixtures/historical_trees.json
      the eleven bundled sources, compiled with the decompile corpus's parameters. Their
      trees are those of docs/reports/batch-10/bundled-after.json.
  docs/reports/2026-10-21-historical-sweep/trees.json
      the nine swept sources, compiled with the parameters of the S02/I04 sweeps.

Every tree is compiled again by the old `ergo-es compile`, and the script stops unless it
equals the recorded one: the decompile corpus's frozen `tree_sha256` for the first set, the
recorded `treeSha256` of the withdrawn sweep rows (forge 211c5b5, `git show`) for the second.
"""
import argparse
import hashlib
import json
import pathlib
import subprocess
import sys
import tempfile

ROOT = pathlib.Path(__file__).resolve().parents[3]
FORGE = "211c5b517d4aedd3a5a6e525999d4134af27fa7a"
NODE = "a203cc02f585bcbe13c743aee008222b208f3c82"
COMPILER = {"node": NODE, "forge": FORGE}

# The sources node 55229e00's typer rejects (docs/reports/node-pin-55229e00.md, section 1).
SOURCES = [
    "dexy/bank/payout.es",
    "dexy/lp/pool/mint.es",
    "hodlcoin/phoenix-hodlcoin-contracts/phoenix_v1_hodlerg_bank.es",
    "hodlcoin/phoenix-hodlcoin-contracts/phoenix_v1_hodlerg_proxy.es",
    "hodlcoin/phoenix-hodlcoin-contracts/phoenix_v1_hodltoken_bank.es",
    "hodlcoin/phoenix/phoenix_v1_hodlcoin_bank.es",
    "hodlcoin/phoenix/phoenix_v1_hodlcoin_proxy.es",
    "hodlcoin/phoenix/phoenix_v1_hodltoken_bank.es",
    "hodlcoin/phoenix/phoenix_v1_hodltoken_fee.es",
    "hodlcoin/phoenix/phoenix_v1_hodltoken_feeTest_mainnet.es",
    "hodlcoin/phoenix/phoenix_v1_hodltoken_feeTest_testnet.es",
]
# The sweeps' own parameters give these two a Long, so they still compile and stay swept.
STILL_COMPILE = {"dexy/bank/payout.es", "dexy/lp/pool/mint.es"}

sha = lambda data: hashlib.sha256(data).hexdigest()


def compile_old(ergo_es, contract, params, network):
    """The tree `ergo-es compile` gives, or the error text."""
    with tempfile.NamedTemporaryFile("w", suffix=".json") as handle:
        json.dump(params, handle)
        handle.flush()
        run = subprocess.run(
            [ergo_es, "compile", str(ROOT / "examples/contracts" / contract), "--params",
             handle.name, "--network", network, "--tree-version", "3"],
            capture_output=True, text=True)
    if run.returncode != 0:
        return None, run.stderr.strip()
    line = next(l for l in run.stdout.splitlines() if l.startswith("tree:"))
    return line.split()[1], None


def corpus_network(ergo_es, contract, params):
    """The decompile corpus tries mainnet and retries testnet on a network mismatch."""
    tree, error = compile_old(ergo_es, contract, params, "mainnet")
    if tree is None and "network mismatch" in error:
        tree, error = compile_old(ergo_es, contract, params, "testnet")
        return "testnet", tree, error
    return "mainnet", tree, error


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--old-ergo-es", required=True)
    ergo_es = ap.parse_args().old_ergo_es

    # 1. Decompile-corpus parameters.
    bundled = {r["id"]: r for r in json.loads(
        (ROOT / "docs/reports/batch-10/bundled-after.json").read_text())}
    expected = json.loads(
        (ROOT / "ergo-sandbox/tests/fixtures/decompile_corpus_expectations.json").read_text())
    rows = []
    for contract in SOURCES:
        ident = "examples/contracts/" + contract
        bundled_row = bundled[ident]
        tree = bundled_row["tree"]
        assert sha(bytes.fromhex(tree)) == expected[ident]["tree_sha256"], ident
        network, again, error = corpus_network(ergo_es, contract, bundled_row["params"])
        assert again == tree, f"{ident}: the old compiler gives {again or error}"
        rows.append({
            "source": ident,
            "sourceSha256": sha((ROOT / ident).read_bytes()),
            "params": bundled_row["params"],
            "network": network,
            "compiler": COMPILER,
            "treeHex": tree,
            "treeSha256": expected[ident]["tree_sha256"],
        })
    out = ROOT / "ergo-sandbox/tests/fixtures/historical_trees.json"
    out.write_text(json.dumps({"rows": rows}, indent=2) + "\n")
    print(f"{out.relative_to(ROOT)}: {len(rows)} rows, each reproduced by the old compiler")

    # 2. The S02/I04 sweeps' parameters, from the rows recorded at 211c5b5.
    recorded = json.loads(subprocess.run(
        ["git", "-C", str(ROOT), "show", f"{FORGE}:docs/reports/batch-7/audit-sweep.json"],
        capture_output=True, text=True, check=True).stdout)
    by_contract = {r["contract"]: r for r in recorded["results"]}
    rows = []
    for contract in SOURCES:
        if contract in STILL_COMPILE:
            continue
        row = by_contract[contract]
        assert row["status"] == "complete", contract
        assert row["sourceSha256"] == sha((ROOT / "examples/contracts" / contract).read_bytes())
        tree, error = compile_old(ergo_es, contract, row["params"], row["network"])
        assert tree and sha(bytes.fromhex(tree)) == row["treeSha256"], f"{contract}: {error}"
        rows.append({
            "contract": contract,
            "sourceSha256": row["sourceSha256"],
            "params": row["params"],
            "network": row["network"],
            "compiler": COMPILER,
            "treeHex": tree,
            "treeSha256": row["treeSha256"],
        })
    out = ROOT / "docs/reports/2026-10-21-historical-sweep/trees.json"
    out.write_text(json.dumps({"rows": rows}, indent=2) + "\n")
    print(f"{out.relative_to(ROOT)}: {len(rows)} rows, each equal to its recorded treeSha256")


if __name__ == "__main__":
    sys.exit(main())
