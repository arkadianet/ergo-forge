"use strict";
const test = require("node:test");
const assert = require("node:assert/strict");
const fs = require("node:fs");
const path = require("node:path");
const { parseHTML } = require("linkedom");
const { bind, render } = require("../attack.js");

const root = path.resolve(__dirname, "../..");
function page() {
  return parseHTML(fs.readFileSync(path.join(root, "ui/index.html"), "utf8")).document;
}
function visible(el) {
  for (let p = el; p; p = p.parentElement) if (p.hidden) return "";
  return el.textContent;
}

// "The USE drain in four moves": a stubbed attack route returns the diff that a
// reorder into the drain order produces. The walkthrough must show which
// scripts accepted, and must label the result synthetic — never "exploit".
test("attack_walkthrough_shows_synthetic_verdict_diff", async () => {
  const document = page();
  document.getElementById("play").hidden = false;
  const draft = {
    height: 1868204,
    network: "mainnet",
    boxes: [],
    tx: { inputs: [{ boxId: "a".repeat(64) }, { boxId: "b".repeat(64) }], dataInputs: [], outputs: [] },
  };
  document.getElementById("attack-draft").value = JSON.stringify(draft);

  const stub = async (_url, opts) => {
    const body = JSON.parse(opts.body);
    assert.ok(Array.isArray(body.operations), "operations are sent");
    return {
      ok: true,
      json: async () => ({
        nodeValidated: false,
        method: "synthetic-play-experiment",
        diff: [
          { boxId: "b".repeat(64), before: "error", after: "pass", changed: true },
          { boxId: "a".repeat(64), before: "pass", after: "pass", changed: false },
        ],
      }),
    };
  };

  const drive = bind(document, stub);
  await drive(JSON.stringify(draft), [{ op: "reorderInputs", order: [1, 0] }]);

  const result = document.getElementById("attack-result");
  const text = visible(result);
  assert.match(text, /error → pass/, "shows the swap script flipping to pass under the reorder");
  assert.match(text, /changed/, "marks the changed verdict");
  assert.match(visible(document.getElementById("attack-message")), /[Ss]ynthetic/, "labels the result synthetic");
  assert.doesNotMatch(text, /exploit|confirmed-violation|node-accepted/, "never claims an exploit");
});

// A render with no diff still shows the synthetic note and never errors.
test("attack_render_is_always_labelled_synthetic", () => {
  const document = page();
  document.getElementById("play").hidden = false;
  const root = document.getElementById("attack-result");
  render({ nodeValidated: false, diff: [] }, root);
  assert.match(visible(root), /[Ss]ynthetic/);
});
