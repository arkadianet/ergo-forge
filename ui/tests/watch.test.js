"use strict";
const test = require("node:test");
const assert = require("node:assert/strict");
const fs = require("node:fs");
const path = require("node:path");
const vm = require("node:vm");
const { parseHTML } = require("linkedom");
const Watch = require("../watch.js");
const app = fs.readFileSync(path.join(__dirname, "../app.js"), "utf8");
const nft = "11".repeat(32), secondNft = "33".repeat(32);
const lock = { schemaVersion: 1, treeHex: "0008d3" };
const baselineBox = { boxId: "22".repeat(32), ergoTree: "0008d3", value: 1000000, additionalRegisters: { R4: "040e" } };
const limitation = JSON.parse(fs.readFileSync(path.join(__dirname, "../../ergo-sandbox/src/identity.rs"), "utf8").match(/pub const LIMITATION: &str = (".*");/)[1]);
function page() { return parseHTML(fs.readFileSync(path.join(__dirname, "../index.html"), "utf8")).document; }
function report(status = "bytes_match", registerStatus = "unchanged", token = nft) {
  return {
    nft: token, lockfileFingerprint: "aa".repeat(32), chainSource: { kind: "fixture", url: "https://fixture.invalid" },
    height: status === "unverified" ? null : 321,
    live: { status, boxId: status === "unverified" ? null : baselineBox.boxId, reason: status === "unverified" ? "no explorer configured (explorer-dependency)" : "Compared locked bytes with supplied box bytes." },
    registers: [{ name: "R4", status: registerStatus, current: registerStatus === "changed" ? "0410" : "040e", expected: "040e", reason: "Serialized register byte observation <img src=x>" }],
    baselineBox: status === "unverified" ? null : baselineBox, baselineOrigin: "first_observation", limitation,
    observation: "Observation of the source's response; chain membership and currentness are not independently verified. Nothing is signed or broadcast.",
  };
}
function payload() { return { watches: [{ lockfile: lock, nfts: [nft], registers: ["R4"], baselineBoxes: {} }] }; }
const response = reports => ({ ok: true, json: async () => reports });

// Uses the actual Read pane and production standalone module, without network.
test("watch renders byte facts, register statuses, provenance and reasons as text", () => {
  const root = page().getElementById("watch-result");
  for (const [bytes, register] of [["bytes_match", "unchanged"], ["bytes_differ", "changed"], ["unverified", "unverified"]]) {
    Watch.render([report(bytes, register), report("bytes_match", "unchanged", secondNft)], root);
    assert.equal(root.querySelectorAll("[data-nft]").length, 2);
    assert.equal(root.querySelector("[data-nft]").dataset.status, bytes);
    assert.equal(root.querySelector("tbody tr").dataset.status, register);
    assert.match(root.textContent, /Lock fingerprint: a{64}/);
    assert.match(root.textContent, /https:\/\/fixture.invalid/);
    assert.match(root.textContent, /not independently verified/);
    assert.match(root.textContent, /Nothing is signed or broadcast/);
    assert.ok(root.textContent.includes(limitation));
    assert.ok(root.textContent.includes(nft));
    assert.equal(root.querySelectorAll("img, script").length, 0);
    assert.doesNotMatch(root.textContent, /\b(safe|compromised)\b/i);
    if (bytes === "unverified") assert.match(root.textContent, /explorer-dependency/);
    if (register === "changed") assert.match(root.textContent, /0410/);
  }
  for (const invalid of [[], [{}], [report("unknown")], [{ ...report(), height: null }]]) {
    assert.throws(() => Watch.render(invalid, root), /Incomplete watch response/);
  }
});

test("watch rejects stale, failed, incomplete and invalidated responses", async () => {
  const root = page().getElementById("watch-result");
  let finish;
  const old = Watch.load(payload(), root, { request: () => new Promise(resolve => { finish = resolve; }) });
  await Watch.load(payload(), root, { request: async (url, options) => {
    assert.equal(url, "/api/v1/watch"); assert.equal(options.method, "POST");
    assert.deepEqual(JSON.parse(options.body), payload());
    return response([report("bytes_differ", "changed")]);
  } });
  finish(response([report()])); await old;
  assert.equal(root.querySelector("[data-nft]").dataset.status, "bytes_differ");
  for (const request of [async () => ({ ok: false, json: async () => ({ error: { message: "invalid lock" } }) }), async () => response([]), async () => { throw new Error("transport failed"); }]) {
    await Watch.load(payload(), root, { request });
    assert.equal(root.querySelectorAll("[data-nft]").length, 0);
    assert.match(root.textContent, /Watch unavailable/);
  }
  const pending = Watch.load(payload(), root, { request: () => new Promise(resolve => { finish = resolve; }) });
  Watch.invalidate(root); finish(response([report()])); await pending;
  assert.equal(root.querySelectorAll("[data-nft]").length, 0);
});

test("Read watch controls gate explorer access and retain a separate baseline per NFT", async () => {
  const document = page(), get = id => document.getElementById(`watch-${id}`);
  const requests = [];
  let fail = false;
  const module = Watch.mount(get("panel"), { request: async (url, options) => {
    const body = JSON.parse(options.body); requests.push(body);
    if (fail) throw new Error("source unavailable");
    return response(body.watches[0].nfts.map(n => ({ ...report(requests.length > 1 ? "bytes_differ" : "bytes_match", "unchanged", n), baselineBox: { ...baselineBox, boxId: n, additionalRegisters: { R4: n === nft ? "040e" : "0410" } } })));
  } });
  assert.equal(get("run").disabled, true);
  assert.match(get("availability").textContent, /no explorer configured \(explorer-dependency\)/);
  await module.run(); assert.equal(requests.length, 0);
  get("lockfile").value = JSON.stringify(lock);
  get("nfts").value = `${nft},\n${secondNft}`;
  get("registers").value = "R4";
  module.configure({ explorer: true, network: "testnet" });
  assert.equal(get("run").disabled, false);
  assert.match(get("availability").textContent, /testnet/);
  await module.run(); await module.run();
  assert.deepEqual(requests[0].watches[0].baselineBoxes, {});
  assert.deepEqual(requests[1].watches[0].baselineBoxes[nft].additionalRegisters, { R4: "040e" });
  assert.deepEqual(requests[1].watches[0].baselineBoxes[secondNft].additionalRegisters, { R4: "0410" });
  assert.equal(Object.hasOwn(requests[0], "explorerUrl"), false);
  fail = true; await module.run(); fail = false; await module.run();
  assert.deepEqual(requests[3].watches[0].baselineBoxes, requests[1].watches[0].baselineBoxes);
  get("reset").click(); await module.run();
  assert.deepEqual(requests.at(-1).watches[0].baselineBoxes, {});
  get("baseline").value = JSON.stringify(baselineBox);
  get("baseline").dispatchEvent(new document.defaultView.Event("input"));
  await module.run();
  assert.deepEqual(requests.at(-1).watches[0].baselineBoxes[nft], baselineBox);
  get("baseline").value = "";
  get("lockfile").dispatchEvent(new document.defaultView.Event("input"));
  await module.run(); assert.deepEqual(requests.at(-1).watches[0].baselineBoxes, {});
  module.configure({ explorer: false }); assert.equal(get("run").disabled, true);
});

test("watch upload, input edits and stale completions cannot preserve old observations", async () => {
  const document = page(), get = id => document.getElementById(`watch-${id}`);
  let finish;
  const module = Watch.mount(get("panel"), { request: () => new Promise(resolve => { finish = resolve; }) });
  module.configure({ explorer: true });
  Object.defineProperty(get("file"), "files", { configurable: true, value: [{ size: 20, text: async () => JSON.stringify(lock) }] });
  await get("file").onchange(); assert.deepEqual(JSON.parse(get("lockfile").value), lock);
  get("nfts").value = nft; get("registers").value = "R4";
  const pending = module.run();
  assert.equal(get("run").disabled, true);
  get("nfts").dispatchEvent(new document.defaultView.Event("input"));
  finish(response([report()])); await pending;
  assert.equal(get("result").querySelectorAll("[data-nft]").length, 0);
  assert.equal(get("run").disabled, false);
  get("registers").value = "R10"; await module.run(); assert.match(get("result").textContent, /Register names/);
  get("registers").value = "R4 R4"; await module.run(); assert.match(get("result").textContent, /Register names must be unique/);
  get("registers").value = "R4";
  get("nfts").value = `${nft} ${nft.toUpperCase()}`; await module.run(); assert.match(get("result").textContent, /NFT IDs must be unique/);
  get("nfts").value = Array.from({ length: 65 }, (_, i) => i.toString(16).padStart(64, "0")).join(" ");
  await module.run(); assert.match(get("result").textContent, /At most 64 NFTs/);
  get("nfts").value = nft;
  let upload;
  Object.defineProperty(get("file"), "files", { configurable: true, value: [{ size: 20, text: () => new Promise(resolve => { upload = resolve; }) }] });
  const uploading = get("file").onchange();
  assert.equal(get("run").disabled, true);
  assert.equal(get("lockfile").value, "");
  assert.equal(await module.run(), undefined, "upload cannot query the previous lock");
  get("lockfile").value = "new text";
  get("lockfile").dispatchEvent(new document.defaultView.Event("input"));
  upload(JSON.stringify(lock)); await uploading;
  assert.equal(get("lockfile").value, "new text");
  Object.defineProperty(get("file"), "files", { configurable: true, value: [{ size: 1024 * 1024 + 1, text() { throw new Error("must not read"); } }] });
  await get("file").onchange(); assert.match(get("result").textContent, /exceeds 1 MiB/);
  assert.equal(get("lockfile").value, "", "failed upload cannot leave a previous lock selected");
  assert.equal(get("run").disabled, false);
});

test("production app mounts Watch in Read and applies the instance configuration", async () => {
  const document = page();
  let mounted, configured;
  const cfg = { explorer: true, network: "mainnet" };
  const context = vm.createContext({ document, $: id => document.getElementById(id),
    Watch: { mount: root => { mounted = root; return { configure: value => { configured = value; } }; } },
    fetch: async url => { assert.equal(url, "/api/v1/config"); return { ok: true, json: async () => cfg }; } });
  vm.runInContext("let explorerConfig = {};", context);
  const start = app.indexOf('const watchPanel = Watch.mount');
  vm.runInContext(app.slice(start, app.indexOf("async function useFetchedBox", start)), context);
  await vm.runInContext("loadConfig()", context);
  assert.equal(mounted.id, "watch-panel");
  assert.equal(mounted.closest("#read").id, "read");
  assert.equal(configured, cfg);
  assert.ok(document.querySelector('script[src="watch.js"]'));
  assert.doesNotMatch(fs.readFileSync(path.join(__dirname, "../watch.js"), "utf8"), /localStorage|sessionStorage|https?:\/\/|innerHTML/);
});
