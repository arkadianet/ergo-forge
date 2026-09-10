"use strict";
const test = require("node:test");
const assert = require("node:assert/strict");
const { renderHunt, renderPreflight, renderTriage } = require("../claim-labels.js");
const { parseHTML } = require("linkedom");
const fs = require("node:fs");
const path = require("node:path");
// Render into elements parsed from the shipped page, using production renderers.
function elements() {
  const { document } = parseHTML(fs.readFileSync(path.join(__dirname, "../index.html"), "utf8"));
  document.getElementById("result").hidden = false;
  return document;
}
function visibleText(el) {
  for (let p = el; p; p = p.parentElement) if (p.hidden) return "";
  return el.textContent;
}
for (const verdict of ["spendableByAnyone", "movableByAnyone", "notUnderProbes", "requiresProof"]) {
  test(`Read and Write render synthetic warning for ${verdict}`, () => {
    const document = elements();
    const label = document.getElementById("hunt-verdict"), warning = document.getElementById("hunt-synthetic");
    warning.hidden = true;
    renderHunt({ verdict, selfSynthetic: true }, label, warning);
    assert.equal(warning.hidden, false);
    assert.match(visibleText(warning), /SELF was synthetic/);
    assert.match(visibleText(label), /full node validation has not run/);
    const write = document.getElementById("c-hunt");
    document.getElementById("compiled").hidden = false;
    document.getElementById("mode-write").hidden = false;
    renderHunt({ verdict, selfSynthetic: true }, write);
    assert.match(visibleText(write), /SELF was synthetic/);
    assert.doesNotMatch(visibleText(label), /Spendable by anyone|see who below/);
    renderHunt({ verdict, selfSynthetic: false }, label, warning);
    assert.equal(visibleText(warning), "");
    assert.match(visibleText(label), /full node validation has not run/);
  });
}
test("Preflight renders success and failure with signature counts, including the legacy alias", () => {
  const document = elements();
  const label = document.getElementById("vtx-verdict");
  document.getElementById("vtx-result").hidden = false;
  for (const field of ["preflightPassed", "valid"]) {
    for (const passed of [true, false]) {
      renderPreflight({ [field]: passed, signaturesNeeded: 2, ergIn: 9, ergOut: 8, height: 7 }, label);
      assert.match(visibleText(label), passed ? /^Preflight passed/ : /^Preflight failed/);
      assert.match(visibleText(label), /2 signature\(s\) needed/);
      assert.match(visibleText(label), /full node validation has not run/);
      assert.doesNotMatch(visibleText(label), /Would validate|Would be rejected/);
    }
  }
});

test("stored v1 confirmation never renders as verified", () => {
  const document = elements();
  const note = document.createElement("div");
  document.body.appendChild(note);
  for (const triage of [{ state: "confirmed" }, { formatVersion: 1, state: "confirmed", nodeValidated: true }, { formatVersion: 2, state: "confirmed", nodeValidated: false }]) {
    renderTriage(triage, note);
    assert.match(visibleText(note), /^Legacy\/unverified record/);
    assert.doesNotMatch(visibleText(note), /^confirmed/);
  }
  renderTriage({ formatVersion: 2, state: "reproduced-in-scenario", nodeValidated: false, explanation: "Sample reached its objective." }, note);
  assert.match(visibleText(note), /^reproduced-in-scenario/);
  assert.match(visibleText(note), /Full node validation has not run/);
});

test("production Read and Write handlers render both signs of synthetic hunt", async () => {
  const vm = require("node:vm");
  const app = fs.readFileSync(path.join(__dirname, "../app.js"), "utf8");
  for (const verdict of ["spendableByAnyone", "notUnderProbes"]) {
    const document = elements();
    document.getElementById("compiled").hidden = false;
    document.getElementById("mode-write").hidden = false;
    const response = { verdict, selfSynthetic: true, probes: [], residuals: [] };
    const context = vm.createContext({ document, ClaimLabels: { renderHunt, renderPreflight, renderTriage }, fetch: async () => ({ ok: true, json: async () => response }) });
    // Execute the shipped reader functions and the complete Write request handler.
    // Extraction only avoids editor/bootstrap side effects; assertions grade DOM output.
    const readerEnd = app.indexOf("// The editor: CodeMirror");
    const writeStart = app.indexOf("async function huntTree(");
    const writeEnd = app.indexOf("async function loadExamples(", writeStart);
    assert.ok(readerEnd > 0 && writeStart > readerEnd && writeEnd > writeStart);
    vm.runInContext(app.slice(0, readerEnd) + app.slice(writeStart, writeEnd), context);
    context.response = response;
    vm.runInContext("renderHunt(response)", context);
    assert.match(visibleText(document.getElementById("hunt-synthetic")), /SELF was synthetic/);
    await vm.runInContext('huntTree("supplied-tree", "mainnet")', context);
    assert.match(visibleText(document.getElementById("c-hunt")), /SELF was synthetic/);
  }
});
