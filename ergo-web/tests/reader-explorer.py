#!/usr/bin/env python3
"""Local synthetic explorer for browser review; no outbound requests."""
import json
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path
from urllib.parse import urlparse

fixture = json.loads((Path(__file__).parents[2] / "ui/examples/reader-map.json").read_text())
boxes = fixture["boxes"].copy()
boxes["33" * 32] = {"boxId": "33" * 32, "ergoTree": "1001040ad191e4c6a704047300", "value": 1000000, "tokens": [], "creationHeight": 1000}
boxes["44" * 32] = {**boxes["22" * 32], "boxId": "44" * 32, "tokens": []}

def explorer_box(box):
    return {**box, "assets": [{"tokenId": t["id"], "amount": t["amount"]} for t in box["tokens"]],
            "additionalRegisters": {"R4": {"serializedValue": "0412"}} if box["boxId"] == "33" * 32 else {}}

class Handler(BaseHTTPRequestHandler):
    def do_GET(self):
        path = urlparse(self.path).path
        value = None
        if path == "/api/v1/networkState":
            value = {"height": fixture["height"]}
        elif path.startswith("/api/v1/boxes/unspent/byAddress/"):
            value = {"items": [explorer_box(boxes[k * 32]) for k in ["33", "44"]], "total": 2}
        elif path.startswith("/api/v1/boxes/unspent/byTokenId/"):
            rec = fixture["boxesByToken"].get(path.split("/")[-1])
            if rec:
                value = {"items": [explorer_box(b) for b in rec["items"]], "total": rec["total"]}
        elif path.startswith("/api/v1/boxes/"):
            box = boxes.get(path.split("/")[-1])
            if box:
                value = explorer_box(box)
        elif path.startswith("/api/v1/tokens/"):
            value = fixture["tokens"].get(path.split("/")[-1])
        self.send_response(200 if value is not None else 404)
        self.send_header("Content-Type", "application/json")
        self.end_headers()
        self.wfile.write(json.dumps(value or {"error": "not found"}).encode())
    def log_message(self, *args):
        pass

if __name__ == "__main__":
    import sys
    ThreadingHTTPServer(("127.0.0.1", int(sys.argv[1]) if len(sys.argv) > 1 else 8101), Handler).serve_forever()
