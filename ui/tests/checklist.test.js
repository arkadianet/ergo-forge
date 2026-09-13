"use strict";
const test = require("node:test");
const assert = require("node:assert/strict");
const fs = require("node:fs");
const path = require("node:path");
const vm = require("node:vm");
const { parseHTML } = require("linkedom");
const Checklist = require("../checklist.js");
const catalogue = require("../../docs/security/vectors.json");
const app = fs.readFileSync(path.join(__dirname, "../app.js"), "utf8");

function page() {
  const { document } = parseHTML(fs.readFileSync(path.join(__dirname, "../index.html"), "utf8"));
  for (const id of ["result", "read", "compiled", "mode-write"]) document.getElementById(id).hidden = false;
  return document;
}
function response() {
  return {
    method: "static-analysis", nodeValidated: false, completeness: "complete",
    rows: catalogue.vectors.map(v => ({ id: v.id, title: v.title, class: v.class, answer: "Unchecked. No supplied experiment or static observation.", provenance: "unchecked", findings: [], artifactFingerprints: [] })),
    artifacts: [],
  };
}

test("checklist shows every unchecked row in Write and Read without a score", () => {
  const document = page();
  for (const id of ["checklist", "c-checklist"]) {
    const root = document.getElementById(id);
    Checklist.render(response(), root);
    assert.equal(root.hidden, false);
    const rows = root.querySelectorAll("[data-vector-id]");
    assert.equal(rows.length, catalogue.vectors.length);
    for (const row of rows) {
      assert.equal(row.hidden, false);
      assert.equal(row.querySelector(".chip").textContent, "unchecked");
    }
    assert.doesNotMatch(root.textContent, /\b(score|grade|ranking|percentage|passed)\b|\d+%/i);
  }
});

test("checklist renders provenance and anchors as text, retaining static findings beside artifacts", () => {
  const document = page();
  const body = response();
  const row = body.rows[0];
  row.provenance = "scenario";
  row.answer = "Supplied scenario reduced to pass. Synthetic; full node validation has not run.";
  row.findings = [{ text: "Reserves read by position <img src=x>", lint: "unbound-box-reserves", anchor: { nodeId: 7, irId: 3 }, snippet: "INPUTS(0).value <script>bad()</script>", provenance: "static" }];
  row.artifactFingerprints = ["abc"];
  body.artifacts = [{ fingerprint: "abc", provenance: "scenario", nodeValidated: false, vectorIds: [row.id], answer: row.answer, result: { nodeValidated: false } }];
  let selected;
  const root = document.getElementById("checklist");
  Checklist.render(body, root, f => { selected = f; });
  const displayed = root.querySelector("[data-vector-id]");
  assert.match(displayed.textContent, /scenario/);
  assert.equal(displayed.querySelector(".checklist-observation .chip").textContent, "static");
  assert.match(displayed.textContent, /node 7 · IR 3/);
  assert.match(displayed.textContent, /INPUTS\(0\).value/);
  assert.equal(root.querySelectorAll("img, script").length, 0);
  displayed.querySelector("button").click();
  assert.equal(selected, row.findings[0]);
});

test("checklist request failures and stale responses cannot display old answers", async () => {
  const root = page().getElementById("checklist");
  let finish;
  const old = Checklist.load("old", "mainnet", root, { request: () => new Promise(resolve => { finish = resolve; }) });
  const body = response();
  body.rows[0].answer = "current answer";
  await Checklist.load("new", "testnet", root, { request: async (url, options) => {
    assert.equal(url, "/api/v1/checklist");
    assert.deepEqual(JSON.parse(options.body), { input: "new", network: "testnet" });
    return { ok: true, json: async () => body };
  } });
  finish({ ok: true, json: async () => response() });
  await old;
  assert.match(root.textContent, /current answer/);
  await Checklist.load("bad", "mainnet", root, { request: async () => ({ ok: false, json: async () => ({ error: { message: "bad input" } }) }) });
  assert.match(root.textContent, /Checklist unavailable: bad input/);
  assert.equal(root.querySelectorAll("[data-vector-id]").length, 0);
});

test("production Read and Write handlers request the checklist for the resolved tree", async () => {
  const document = page();
  const calls = [];
  const mock = { ...Checklist, load: (...args) => { calls.push(args); Checklist.render(response(), args[2]); } };
  const context = vm.createContext({ document, Checklist: mock, AbortController, Event: class Event {}, ClaimLabels: require("../claim-labels.js") });
  vm.runInContext(app.slice(0, app.indexOf("// The editor: CodeMirror")), context);
  // Read the shipped response handler, with transport isolated from the DOM.
  context.body = { source: "sigmaProp(false)", treeHex: "resolved-tree", address: "address", completeness: "complete", plain: [], findings: [], obligations: [] };
  context.fetch = async url => ({ ok: true, json: async () => url.endsWith("/inspect") ? context.body : ({ probes: [], residuals: [] }) });
  document.getElementById("input").value = "address";
  document.getElementById("network").querySelector('option[value="testnet"]').selected = true;
  document.getElementById("read-kind").querySelector('option[value="address"]').selected = true;
  await vm.runInContext("read()", context);
  assert.equal(calls[0][0], "resolved-tree");
  assert.equal(calls[0][1], "testnet");
  assert.equal(calls[0][2].id, "checklist");
  assert.match(document.getElementById("checklist").textContent, /unchecked/);
  const start = app.indexOf("function renderCompiled(c)");
  const end = app.indexOf("async function huntTree(", start);
  vm.runInContext("let compileGeneration = 0; function markFindings() {} function appendTriage() {}", context);
  vm.runInContext(app.slice(start, end), context);
  document.getElementById("write-network").querySelector('option[value="mainnet"]').selected = true;
  vm.runInContext("renderCompiled({ ...body, p2s: 'address', p2sh: 'hash' })", context);
  assert.equal(calls[1][0], "resolved-tree");
  assert.equal(calls[1][1], "mainnet");
  assert.equal(calls[1][2].id, "c-checklist");
});

test("negative space visibly labels every line static and retains anchors below plain words", () => {
  const document = page();
  const root = document.getElementById("negative-space");
  const lines = [{ text: "Data input 0 supplies R4 without a recognised identity comparison.", lint: "trust-assumptions", anchor: { nodeId: 9, irId: null }, snippet: "CONTEXT.dataInputs(0).R4[Long]", provenance: "static" }];
  Checklist.renderNegativeSpace(lines, root);
  assert.equal(root.querySelector(".chip").textContent, "static");
  assert.match(root.textContent, /node 9 · IR unavailable/);
  assert.match(root.textContent, /CONTEXT.dataInputs\(0\).R4\[Long\]/);
  const order = [...document.getElementById("result").children];
  assert.ok(order.indexOf(root) > order.indexOf(document.getElementById("plain")));
  assert.ok(order.indexOf(root) < order.indexOf(document.getElementById("checklist")));
  Checklist.renderNegativeSpace([], root);
  assert.match(root.textContent, /No observations.*does not establish/);
  assert.equal(root.querySelectorAll(".checklist-observation").length, 0);
});
