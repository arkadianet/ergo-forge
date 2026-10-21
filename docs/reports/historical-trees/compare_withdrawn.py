"""Compare the historical sweep with the S02 and I04 rows the typer withdrew.

    python3 docs/reports/historical-trees/compare_withdrawn.py

The withdrawn rows are those of forge 211c5b5, read with `git show`. For each of the nine
contracts the script requires every field but `provenance`, `compiler` and `attemptErrors`
to be equal, so the same tree, the same findings (snippets and messages included) and the
same counts. It exits non-zero otherwise.
"""
import hashlib
import json
import pathlib
import subprocess
import sys

ROOT = pathlib.Path(__file__).resolve().parents[3]
FORGE = "211c5b517d4aedd3a5a6e525999d4134af27fa7a"
ADDED = {"provenance", "compiler", "attemptErrors"}


def withdrawn(batch):
    blob = subprocess.run(
        ["git", "-C", str(ROOT), "show", f"{FORGE}:docs/reports/{batch}/audit-sweep.json"],
        capture_output=True, check=True).stdout
    rows = {r["contract"]: r for r in json.loads(blob)["results"]}
    return hashlib.sha256(blob).hexdigest(), rows


def main():
    new = json.loads((ROOT / "docs/reports/2026-10-21-historical-sweep/audit-sweep.json").read_text())
    sweeps = {"S02": withdrawn("batch-2"), "I04": withdrawn("batch-7")}
    for name, (digest, _) in sweeps.items():
        print(f"{name}: audit-sweep.json at {FORGE[:7]} has SHA-256 {digest}")
    failed = False
    totals = {name: 0 for name in sweeps}
    print(f"{'contract':<68} {'tree':<12} findings  S02  I04  other fields")
    for row in new["results"]:
        cells = []
        for name, (_, rows) in sweeps.items():
            old = rows[row["contract"]]
            differing = {k for k in set(row) | set(old) if row.get(k) != old.get(k)} - ADDED
            failed |= bool(differing)
            totals[name] += len(old["findings"])
            cells.append((old, differing))
        same_tree = all(old["treeSha256"] == row["treeSha256"] for old, _ in cells)
        print(f"{row['contract']:<68} {row['treeSha256'][:12]} {len(row['findings']):>8} "
              f"{len(cells[0][0]['findings']):>4} {len(cells[1][0]['findings']):>4}  "
              f"{'equal' if not any(d for _, d in cells) and same_tree else 'DIFFERENT'}")
    total = sum(len(r["findings"]) for r in new["results"])
    print(f"findings: {total} restored; withdrawn {totals['S02']} (S02), {totals['I04']} (I04)")
    return 1 if failed or total != totals["S02"] or total != totals["I04"] else 0


if __name__ == "__main__":
    sys.exit(main())
