"use strict";
const test = require("node:test");
const assert = require("node:assert/strict");
const fs = require("node:fs");
const path = require("node:path");
const vm = require("node:vm");
const { parseHTML } = require("linkedom");
const Verify = require("../verify.js");
const app = fs.readFileSync(path.join(__dirname, "../app.js"), "utf8");
const limitation = JSON.parse(fs.readFileSync(path.join(__dirname, "../../ergo-sandbox/src/identity.rs"), "utf8").match(/pub const LIMITATION: &str = (".*");/)[1]);
function page() { return parseHTML(fs.readFileSync(path.join(__dirname, "../index.html"), "utf8")).document; }
function report(outcome) { return { outcome, limitation, constantDifferences: outcome === "template" ? [
  { path: [0,1], left: { table_index: 0, type: "Int", value: "Int(100)" }, right: { table_index: 1, type: "Long", value: "Long(200)" } },
  { path: [1,1], left: { table_index: null, type: "Coll[Byte]", value: "<img src=x>" }, right: { table_index: 2, type: "Coll[Byte]", value: "abcdef" } },
] : [] }; }

test("verify renders three distinct outcomes, every typed constant and the full limitation as text", () => {
  const root = page().getElementById("verify-result");
  for (const outcome of ["exact", "template", "no_match"]) {
    Verify.render(report(outcome), root);
    assert.equal(root.dataset.outcome, outcome);
    assert.ok(root.textContent.includes(limitation));
    assert.doesNotMatch(root.textContent, /\b(score|safe)\b/i);
    assert.equal(root.querySelectorAll("li").length, outcome === "template" ? 2 : 0);
    if (outcome === "template") {
      assert.match(root.textContent, /Path \[0,1\]/);
      assert.match(root.textContent, /Long\(200\)/);
      assert.match(root.textContent, /table_index/);
      assert.match(root.textContent, /abcdef/);
    }
    assert.equal(root.querySelectorAll("img, script").length, 0);
  }
});

test("verify rejects stale results and displays failures without inferring a match", async () => {
  const root = page().getElementById("verify-result");
  let finish;
  const pending = Verify.load({ target: "old" }, root, { request: () => new Promise(resolve => { finish = resolve; }) });
  await Verify.load({ target: "new" }, root, { request: async (url, options) => {
    assert.equal(url, "/api/v1/verify"); assert.equal(JSON.parse(options.body).target, "new");
    return { ok: true, json: async () => report("no_match") };
  } });
  finish({ ok: true, json: async () => report("exact") }); await pending;
  assert.equal(root.dataset.outcome, "no_match");
  await Verify.load({}, root, { request: async () => ({ ok: false, json: async () => ({ error: { message: "missing parameters" } }) }) });
  assert.equal(root.dataset.outcome, undefined);
  assert.match(root.textContent, /Verification unavailable: missing parameters/);
  let changed = false;
  await Verify.load({}, root, { isCurrent: () => !changed, request: async () => { changed = true; Verify.invalidate(root); return { ok: true, json: async () => report("exact") }; } });
  assert.equal(root.dataset.outcome, undefined);
});

test("production Write control sends current source, params, network and target and invalidates on changes", async () => {
  const document = page(); const $ = id => document.getElementById(id);
  let supplied;
  const context = vm.createContext({ document, $, Verify: { ...Verify, load: async (payload, root, options) => { supplied = payload; assert.equal(options.isCurrent(), true); Verify.render(report("template"), root); } }, editorValue: () => "current source\n", collectParams: () => ({h:{type:"Int",value:100}}) });
  vm.runInContext("let compileGeneration = 0; let contextGeneration = 0;", context);
  $("verify-target").value = "target";
  $("write-network").querySelector('[value="testnet"]').selected = true;
  const start = app.indexOf("function verifyDeployment()");
  vm.runInContext(app.slice(start, app.indexOf("// ── export: share link", start)), context);
  $("verify-run").click();
  assert.deepEqual(JSON.parse(JSON.stringify(supplied)), {source:"current source\n",params:{h:{type:"Int",value:100}},network:"testnet",target:"target"});
  $("verify-target").dispatchEvent(new document.defaultView.Event("input"));
  assert.equal($("verify-result").dataset.outcome, undefined);
  const listeners = {};
  context.editor = { on: (name, handler) => { listeners[name] = handler; } };
  vm.runInContext("let compileErrorMark = null; function clearCompileError() {}", context);
  context.Checklist = { invalidate() {} };
  const invalidate = app.indexOf("function invalidateCompileError()");
  vm.runInContext(app.slice(invalidate, app.indexOf("function showCaret", invalidate)), context);
  for (const fire of [() => listeners.change(), ...["params-rows", "write-network"].map(id => () => $(id).dispatchEvent(new document.defaultView.Event("change")))]) {
    Verify.render(report("exact"), $("verify-result")); fire();
    assert.equal($("verify-result").dataset.outcome, undefined);
  }
});

test("project export includes the server lock and preserves the STORE zip with the requested snapshot", async () => {
  const document = page(); let zip; const calls = []; let failure = false; const notices = [];
  const locked = {schemaVersion:1,sourceSha256:"digest",treeHex:"0008d3",limitation};
  const context = vm.createContext({ document, $: id => document.getElementById(id), TextEncoder, Blob,
    URL: { createObjectURL(blob) { zip = blob; return "blob:zip"; }, revokeObjectURL() {} }, setTimeout() {},
    toast: text => notices.push(text), readerRequest: async (route, payload) => { calls.push([route, payload]); if (failure) throw new Error("compile rejected"); return locked; } });
  const start = app.indexOf("async function downloadProject(");
  vm.runInContext(app.slice(start, app.indexOf("// Verification always compiles", start)), context);
  context.source = "// λ\r\nsigmaProp(true)\n";
  assert.equal(await vm.runInContext('downloadProject(source, {h:{type:"Int",value:100}}, "testnet", "contract")', context), true);
  assert.equal(calls[0][0], "lock"); assert.equal(calls[0][1].source, context.source);
  assert.equal(calls[0][1].network, "testnet");
  const bytes = Buffer.from(await zip.arrayBuffer()); const files = {}; let offset = 0;
  while (bytes.readUInt32LE(offset) === 0x04034b50) {
    assert.equal(bytes.readUInt16LE(offset + 8), 0, "STORE method");
    const size = bytes.readUInt32LE(offset + 18), nameSize = bytes.readUInt16LE(offset + 26), extra = bytes.readUInt16LE(offset + 28);
    const name = bytes.subarray(offset + 30, offset + 30 + nameSize).toString();
    offset += 30 + nameSize + extra;
    files[name] = bytes.subarray(offset, offset + size).toString(); offset += size;
  }
  assert.equal(bytes.readUInt32LE(offset), 0x02014b50);
  assert.deepEqual(JSON.parse(files["contract.lock.json"]), locked);
  assert.equal(files["contract.es"], context.source);
  assert.equal(JSON.parse(files["contract.test.json"]).source, context.source);
  assert.equal(JSON.parse(files["params.json"]).h.value, 100);
  assert.match(files["README.md"], /verify-lock.*--network testnet/);
  failure = true; zip = undefined;
  assert.equal(await vm.runInContext('downloadProject("bad", {}, "mainnet", "contract")', context), false);
  assert.equal(zip, undefined); assert.match(notices[0], /Project export failed: compile rejected/);
});
