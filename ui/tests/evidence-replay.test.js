"use strict";
const test = require("node:test");
const assert = require("node:assert/strict");
const fs = require("node:fs");
const path = require("node:path");
const os = require("node:os");
const { spawnSync } = require("node:child_process");
const { parseHTML } = require("linkedom");
const { bind } = require("../evidence-replay.js");
const { renderPreflight } = require("../claim-labels.js");
const root = path.resolve(__dirname, "../..");
function fixture(name) {
  return JSON.parse(fs.readFileSync(path.join(root, `ergo-sandbox/tests/fixtures/evidence/claim-vectors/${name}.fixture`), "utf8"));
}
function page() {
  return parseHTML(fs.readFileSync(path.join(root, "ui/index.html"), "utf8")).document;
}
function visible(el) {
  for (let p = el; p; p = p.parentElement) if (p.hidden) return "";
  return el.textContent;
}
let cli = process.env.ERGO_REPLAY_CLI;
function replay(bundle) {
  if (!cli) {
    const built = spawnSync("cargo", ["build", "--release", "-p", "ergo-sandbox", "--bin", "ergo-es"], {
      cwd: root, encoding: "utf8", env: { ...process.env, CARGO_TARGET_DIR: process.env.CARGO_TARGET_DIR || "./target-p00" },
    });
    assert.equal(built.status, 0, built.stderr);
    cli = path.resolve(root, process.env.CARGO_TARGET_DIR || "./target-p00", "release/ergo-es");
  }
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), "p08-ui-"));
  try {
    const file = path.join(dir, "bundle.json");
    fs.writeFileSync(file, JSON.stringify(bundle));
    const output = spawnSync(cli, ["replay", file, "--json"], { cwd: root, encoding: "utf8" });
    assert.ok(output.status === 0 || output.status === 1, output.stderr);
    return { apiVersion: 2, result: JSON.parse(output.stdout) };
  } finally { fs.rmSync(dir, { recursive: true }); }
}
test("historical_and_hypothetical_state_labels_are_visible", async () => {
  const incomplete = fixture("sale-fixed-paid");
  incomplete.execution.headers = { status: "missing", reason: "headers deliberately omitted for P08 replay" };
  incomplete.execution.parameters = { status: "missing", reason: "parameters deliberately omitted for P08 replay" };
  for (const [bundle, status, origin] of [
    [fixture("use-incident"), "confirmed-violation", "source-recorded"],
    [fixture("sale-fixed-paid"), "accepted-nonviolating", "hypothetical"],
    [incomplete, "incomplete-or-invalid-premises", "hypothetical"],
  ]) {
    const response = replay(bundle);
    assert.equal(response.result.status, status);
    const document = page();
    document.getElementById("replay-file").files = [{ text: async () => JSON.stringify(bundle) }];
    const calls = [];
    const click = bind(document, async (url, options) => {
      calls.push(url);
      assert.equal(options.method, "POST");
      assert.deepEqual(JSON.parse(options.body), bundle);
      return { ok: true, json: async () => response };
    });
    await click();
    assert.deepEqual(calls, ["/api/v2/replay"]);
    const result = document.getElementById("replay-result");
    const text = visible(result);
    assert.ok(text.includes(response.result.scope));
    if (response.result.claim) assert.ok(text.includes(response.result.claim.scope));
    assert.match(text, /Historical state: Not established/);
    assert.ok(text.includes(origin));
    if (origin === "hypothetical") assert.match(text, /Hypothetical state: Includes hypothetical inputs/);
    const expectedMissing = [];
    function walk(value, location) {
      if (!value || typeof value !== "object") return;
      if (value.status === "missing") expectedMissing.push(`${location}: ${value.reason}`);
      for (const [key, child] of Object.entries(value)) walk(child, `${location}.${key}`);
    }
    walk(bundle, "bundle");
    assert.deepEqual([...result.querySelectorAll(".replay-missing-premises li")].map(el => visible(el)).sort(), expectedMissing.sort());
    assert.equal(result.querySelectorAll(".node-claim-badge").length, status === "confirmed-violation" ? 1 : 0);
  }
});
test("legacy_preflight_does_not_use_node_claim_badge", () => {
  const document = page();
  document.getElementById("vtx-result").hidden = false;
  for (const valid of [true, false]) {
    const el = document.getElementById("vtx-verdict");
    renderPreflight({ valid, nodeValidated: true, signaturesNeeded: 0, ergIn: 1, ergOut: 1, height: 1 }, el);
    assert.match(visible(el), /full node validation has not run/);
    assert.equal(document.querySelectorAll(".node-claim-badge").length, 0);
  }
});
test("saved verdicts are discarded and errors clear earlier claims", async () => {
  const document = page();
  const bundle = fixture("sale-fixed-paid");
  let saved = JSON.stringify({ bundle, status: "confirmed-violation", nodeValidated: true });
  document.getElementById("replay-file").files = [{ text: async () => saved }];
  let calls = 0;
  const click = bind(document, async (url, options) => {
    calls++;
    assert.equal(url, "/api/v2/replay");
    assert.deepEqual(JSON.parse(options.body), bundle);
    return { ok: false, json: async () => ({ error: { message: "Invalid evidence" } }) };
  });
  await click();
  assert.equal(visible(document.getElementById("replay-result")), "");
  assert.match(visible(document.getElementById("replay-message")), /Invalid evidence/);
  saved = "not JSON";
  await click();
  assert.equal(calls, 1);
  assert.equal(document.getElementById("replay-go").disabled, false);
});
