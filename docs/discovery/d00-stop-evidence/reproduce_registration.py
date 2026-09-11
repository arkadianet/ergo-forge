"""Reproduce the proposed registration against the unchanged real CI runner.

Temporarily appends exactly the design's five false entries, then restores the
roadmap bytes even if CI fails. This is a governance diagnostic, not a D00 gate.
Run only in an idle workspace; no concurrent roadmap edits are supported.
"""
import hashlib
import json
import os
from pathlib import Path
import re
import subprocess


ROOT = Path(__file__).resolve().parents[3]
HERE = Path(__file__).resolve().parent


def main():
    roadmap = ROOT / "docs/ROADMAP.md"
    original = roadmap.read_bytes()
    design = (ROOT / "docs/superpowers/specs/2026-09-11-discovery-capability-design.md").read_text()
    entries = json.loads(re.search(r"```json\s*(\[.*?\])\s*```", design, re.S)[1])
    assert [u["id"] for u in entries] == [f"D0{i}" for i in range(5)]
    assert all(u["implemented"] is False for u in entries)
    text = original.decode()
    block = re.search(r"<!-- roadmap-policy:v1 -->\s*```json\s*(.*?)\s*```", text, re.S)
    policy = json.loads(block[1])
    assert not any(u["id"].startswith("D") for u in policy["units"])
    policy["units"].extend(entries)
    proposed = text[:block.start(1)] + json.dumps(policy, indent=2) + text[block.end(1):]
    (HERE / "proposed-roadmap.fixture").write_text(proposed)
    command = ["python3", "scripts/roadmap_gate.py", "--ci", "--report",
               "docs/discovery/d00-stop-evidence/proposed-ci.json"]
    try:
        roadmap.write_text(proposed)
        with (HERE / "proposed-ci.log").open("w") as log:
            result = subprocess.run(command, cwd=ROOT, stdout=log, stderr=subprocess.STDOUT,
                                    env={**os.environ, "CARGO_TARGET_DIR": "./target-p00"}, check=False)
    finally:
        roadmap.write_bytes(original)
    report = json.loads((HERE / "proposed-ci.json").read_text())
    states = {r["unit"]: r["status"] for r in report["results"]}
    assert result.returncode == 1
    assert all(states[f"D0{i}"] == "unimplemented" for i in range(5))
    assert states["M00"] == "failed"
    receipt = {"command": command, "exitCode": result.returncode,
               "originalRoadmapSha256": hashlib.sha256(original).hexdigest(),
               "restoredExactly": roadmap.read_bytes() == original,
               "results": report["results"],
               "scope": "Real CI over exact proposed false registrations; no D00 implementation or reference execution measurement."}
    (HERE / "registration-receipt.json").write_text(json.dumps(receipt, indent=2) + "\n")
    print(json.dumps(receipt, indent=2))
    return result.returncode


if __name__ == "__main__":
    raise SystemExit(main())
