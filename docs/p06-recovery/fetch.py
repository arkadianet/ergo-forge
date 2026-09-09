#!/usr/bin/env python3
"""One-time raw-response recorder for P06 box provenance.

Records every request URL, UTC retrieval time, HTTP status, raw response path
and SHA-256. Responses are preserved byte-for-byte. Neither the acceptance test
nor any product code invokes this: refetching is not a gate repair.
"""
import hashlib, json, sys, urllib.request
from datetime import datetime, timezone
from pathlib import Path

HERE = Path(__file__).resolve().parent

def main(pairs):
    out = []
    for name, url in pairs:
        with urllib.request.urlopen(url, timeout=30) as r:
            body, status = r.read(), r.status
        path = HERE / f"{name}.fixture"
        path.write_bytes(body)
        out.append({
            "name": name, "url": url, "status": status,
            "retrievedAt": datetime.now(timezone.utc).isoformat(timespec="seconds"),
            "rawResponse": f"docs/p06-recovery/{name}.fixture",
            "sha256": hashlib.sha256(body).hexdigest(),
        })
    (HERE / "retrievals.json").write_text(json.dumps(out, indent=1) + "\n")
    print(json.dumps(out, indent=1))

if __name__ == "__main__":
    main(json.loads(sys.argv[1]))
