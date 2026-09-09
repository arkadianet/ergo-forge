// Browser regression checks using system Chromium directly; no npm dependencies.
// node ergo-web/tests/browser.mjs [offline URL] [mock-explorer URL]
import { spawn } from 'node:child_process';
import { mkdtemp, mkdir, writeFile, readFile, rm } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join, resolve } from 'node:path';
import assert from 'node:assert/strict';

const offline = process.argv[2] || 'http://127.0.0.1:8099';
const live = process.argv[3] || 'http://127.0.0.1:8100';
const shots = resolve('ui/ux-review/screenshots');
await mkdir(shots, { recursive: true });
const profile = await mkdtemp(join(tmpdir(), 'ergo-reader-'));
const chrome = spawn('/usr/bin/chromium-browser', ['--headless', '--no-sandbox', '--disable-gpu', '--remote-debugging-port=0', `--user-data-dir=${profile}`, '--window-size=1440,1100', 'about:blank']);
let stderr = '';
const endpoint = await new Promise((resolve, reject) => {
  const timeout = setTimeout(() => reject(new Error(stderr || 'Chromium did not start')), 10000);
  chrome.stderr.on('data', data => {
    stderr += data;
    const match = stderr.match(/DevTools listening on (ws:\/\/[^\s]+)/);
    if (match) { clearTimeout(timeout); resolve(match[1]); }
  });
  chrome.on('error', reject);
});
const port = new URL(endpoint).port;
const pages = await (await fetch(`http://127.0.0.1:${port}/json`)).json();
const ws = new WebSocket(pages.find(p => p.type === 'page').webSocketDebuggerUrl);
await new Promise(r => ws.addEventListener('open', r, { once: true }));
let sequence = 0;
const pending = new Map();
const exceptions = [];
ws.addEventListener('message', e => {
  const m = JSON.parse(e.data);
  if (m.id) { pending.get(m.id)?.(m); pending.delete(m.id); }
  if (m.method === 'Runtime.exceptionThrown') exceptions.push(m.params.exceptionDetails);
});
const send = (method, params = {}) => new Promise((resolve, reject) => {
  const id = ++sequence;
  const timeout = setTimeout(() => { pending.delete(id); reject(new Error(`CDP timeout: ${method}`)); }, 15000);
  pending.set(id, m => { clearTimeout(timeout); m.error ? reject(new Error(JSON.stringify(m.error))) : resolve(m.result); });
  ws.send(JSON.stringify({ id, method, params }));
});
async function evaluate(expression) {
  const res = await send('Runtime.evaluate', { expression, awaitPromise: true, returnByValue: true });
  if (res.exceptionDetails) throw new Error(res.exceptionDetails.exception?.description || res.exceptionDetails.text);
  return res.result.value;
}
const wait = expression => evaluate(`(async()=>{for(let i=0;i<150;i++){if(${expression}) return true;await new Promise(r=>setTimeout(r,50));}throw Error(${JSON.stringify('Timed out: ' + expression)});})()`);
async function navigate(url) {
  await send('Page.navigate', { url });
  await wait('document.readyState === "complete" && typeof configReady !== "undefined"');
  await evaluate('configReady');
}
async function screenshot(name) {
  const shot = await send('Page.captureScreenshot', { captureBeyondViewport: false });
  await writeFile(join(shots, `${name}.png`), Buffer.from(shot.data, 'base64'));
}
try {
  await send('Runtime.enable');
  await send('Page.enable');
  await mkdir(join(profile, 'downloads'));
  await send('Page.setDownloadBehavior', { behavior: 'allow', downloadPath: join(profile, 'downloads') });
  await send('Emulation.setDeviceMetricsOverride', { width: 1440, height: 1100, deviceScaleFactor: 1, mobile: false });
  await navigate(offline);
  assert.equal(await evaluate('document.querySelector(".mode.active").textContent'), 'Read');
  assert.equal(await evaluate('$("dev-panels").parentElement.id'), 'read-dev-slot');
  await screenshot('after-entry');
  await evaluate('$("mode-read").focus();$("mode-read").dispatchEvent(new KeyboardEvent("keydown",{key:"ArrowRight",bubbles:true}))');
  assert.equal(await evaluate('document.activeElement.id'), 'mode-build');
  await evaluate('setMode("read")');
  await evaluate('$("input").value="33".repeat(32); read()');
  assert.match(await evaluate('$("status").textContent'), /box ID.*unavailable/);
  await screenshot('after-box-id-offline');
  await evaluate('$("examples").value="1001040ad191e4c6a704047300"; $("examples").dispatchEvent(new Event("change"))');
  await wait('lastRead && $("hunt-probes").children.length === 6');
  assert.match(await evaluate('$("source").textContent'), /SELF.R4/);
  await evaluate('window.scrollTo(0,0)');
  await screenshot('after-read');
  // Author A, read B: neither Scenario nor Tests may use A in Read mode.
  await evaluate(String.raw`(async()=>{setMode("write");editor.setValue("sigmaProp(HEIGHT > 1000)");await compile();showTab("scenario");$("scenario").value="{\"height\":1001}";await runScenario();})()`);
  assert.match(await evaluate('$("eval-verdict").textContent'), /^PASS/);
  await evaluate('(async()=>{showTab("tests");$("tests").value=JSON.stringify([{name:"before",expect:"fail",height:1000},{name:"after",expect:"pass",height:1001}]);await runTests();})()');
  assert.equal(await evaluate('$("tests-summary").textContent'), '2 passed, 0 failed');
  await evaluate(String.raw`(async()=>{setMode("read");$("read-test").click();$("scenario").value="{\"height\":1001}";await runScenario();$("tests").value=JSON.stringify([{name:"missing register",expect:"error",height:1001}]);await runTests();})()`);
  assert.match(await evaluate('$("eval-verdict").textContent'), /^ERROR/);
  assert.equal(await evaluate('$("tests-summary").textContent'), '1 passed, 0 failed');
  assert.equal(await evaluate('currentSuite().suite.tree'), '1001040ad191e4c6a704047300');
  await screenshot('after-reader-tests');
  await evaluate(`(async()=>{const real=window.fetch;window.fetch=async(...args)=>{const r=await real(...args);if(String(args[0]).endsWith('/eval')) await new Promise(r=>setTimeout(r,150));return r;};const run=runScenario();setMode('write');await run;window.fetch=real;})()`);
  assert.equal(await evaluate('$("eval-result").hidden'), true);
  await evaluate('setMode("read")');
  // Read edit invalidates stale source/context immediately.
  await evaluate('$("input").value="bad";$("input").dispatchEvent(new Event("input"));runScenario()');
  assert.equal(await evaluate('lastRead'), null);
  assert.match(await evaluate('$("eval-status").textContent'), /Read a contract first/);
  await evaluate('$("reader-tools").open=false;$("open-map").click();$("map-demo").click()');
  await wait('mapResult !== null');
  assert.equal(await evaluate('mapResult.nodes.length'), 2);
  assert.equal(await evaluate('document.querySelectorAll("#map-edges tr").length'), 2);
  await evaluate('$("protocol-map").scrollIntoView()');
  await screenshot('after-map');
  await evaluate('$("map-export").click()');
  let exported;
  for (let attempt = 0; attempt < 30; attempt++) {
    try { exported = JSON.parse(await readFile(join(profile, 'downloads/protocol-map.json'), 'utf8')); break; }
    catch { await new Promise(r => setTimeout(r,50)); }
  }
  assert.deepEqual(exported, await evaluate('mapResult'), 'download preserves the displayed map');
  await evaluate('const upload=new DataTransfer();upload.items.add(new File([JSON.stringify(mapFixture)],"recording.json",{type:"application/json"}));$("map-file").files=upload.files;$("map-file").dispatchEvent(new Event("change"))');
  await wait('$("map-context").textContent.startsWith("recording.json")');
  assert.equal(await evaluate('mapResult'), null);
  await evaluate('runMap()');
  assert.equal(await evaluate('mapResult.nodes.length'), 2);
  await evaluate('$("map-nodes").value="1";runMap()');
  assert.equal(await evaluate('mapResult.truncated.nodes'), 1);
  assert.match(await evaluate('$("map-limits").textContent'), /omitted/);
  assert.equal(await evaluate('mapResult.edges.some(e=>e.unresolved != null)'), true);
  await screenshot('after-map-limits');
  await evaluate('$("map-nodes").value="24";runMap()');
  await evaluate('document.querySelectorAll("#map-boxes button")[1].click()');
  await wait('lastRead?.treeHex === mapResult.nodes[1].treeHex');
  assert.match(await evaluate('$("source").textContent'), /HEIGHT > 1000/);
  assert.equal(await evaluate('$("self-box").value'), '');
  // A delayed response cannot overwrite a newer read or map.
  await evaluate(`(async()=>{const real=window.fetch;window.fetch=async(...args)=>{const r=await real(...args);if(String(args[0]).endsWith('/inspect')) await new Promise(r=>setTimeout(r,150));return r;};$("input").value="1001040ad191e4c6a704047300";const first=read();$("input").value="100104d00fd191a37300";const second=read();await Promise.all([first,second]);window.fetch=real;})()`);
  assert.match(await evaluate('$("source").textContent'), /HEIGHT > 1000/);
  await evaluate(`(async()=>{const real=window.fetch;window.fetch=async(...args)=>{const r=await real(...args);if(String(args[0]).endsWith('/map')) await new Promise(r=>setTimeout(r,150));return r;};const first=runMap();$("network").value="testnet";$("network").dispatchEvent(new Event('change'));await first;window.fetch=real;})()`);
  assert.equal(await evaluate('mapResult'), null);
  assert.equal(await evaluate('$("map-go").disabled'), false);
  // Build, examples, compile errors, point derivation, Play, validation.
  await evaluate('setMode("build")');
  await wait('$("recipes").children.length > 0');
  await evaluate('document.querySelector(".tour-start[data-tour=lock]").click()');
  await wait('!$("tour-next").disabled');
  await evaluate('$("tour-stop").click();setMode("write");$("example-pick").value=[...$("example-pick").options].find(x=>x.textContent==="height-lock").value;$("example-pick").dispatchEvent(new Event("change"))');
  await wait('editor.getValue().includes("$unlockHeight")');
  await evaluate('editor.setValue("sigmaProp(");compile()');
  assert.equal(await evaluate('$("compile-status").hidden'), false);
  await evaluate(String.raw`(async()=>{editor.setValue("proveDlog(decodePoint(fromBase16(\"0279be667ef9dcbbac55a06295ce870b07029bfcdb2dce28d959f2815b16f81798\")))");await compile();showTab("scenario");$("scenario").value="{\"height\":1000}";await runScenario();$("prove-add-dlog").click();const input=document.querySelector(".p-secret");input.value="0".repeat(63)+"1";input.dispatchEvent(new Event("change"));})()`);
  await wait('document.querySelector(".derived").textContent.includes("0279be667e")');
  await evaluate('setMode("play");$("play-fund-toggle").click();$("fund-contract").value="10010101d17300";$("fund-go").click()');
  await wait('document.querySelector("#play-boxes button")');
  await evaluate('document.querySelector("#play-boxes button").click()');
  await wait('!$("play-tx").hidden && document.querySelector(".o-dest")');
  await evaluate('$("tx-send").click()');
  await wait('$("tx-status").textContent.startsWith("Accepted")');
  await evaluate('setMode("write");showTab("validate");$("txjson").value=JSON.stringify({tx:{inputs:[],outputs:[]},boxes:[],height:1200});$("validate-tx").click()');
  await wait('!$("vtx-result").hidden');
  assert.match(await evaluate('$("vtx-verdict").textContent'), /Would validate/);
  await navigate(live);
  await evaluate('$("input").value="33".repeat(32);read()');
  assert.match(await evaluate('$("source").textContent'), /SELF.R4/);
  assert.equal(await evaluate('JSON.parse($("self-box").value).registers.R4.value'), '0412');
  await wait('$("hunt-probes").children.length===6');
  await screenshot('after-live-box');
  await evaluate('$("chain-panel").open=true;$("chain-input").value=lastRead.address;$("chain-fetch").click()');
  await wait('!$("chain-fetch").disabled && fetchedBoxes.length===2');
  await evaluate('$("chain-boxes").value="1";$("chain-boxes").dispatchEvent(new Event("change"))');
  await wait('lastRead?.box?.boxId === "44".repeat(32)');
  assert.match(await evaluate('$("source").textContent'), /HEIGHT > 1000/);
  assert.equal(await evaluate('JSON.parse($("self-box").value).registers.R4'), undefined);
  await evaluate('$("open-map").click();$("map-input").value="11".repeat(32);$("map-kind").value="boxId";runMap()');
  assert.equal(await evaluate('mapResult.source.recorded'), false);
  assert.equal(await evaluate('mapResult.nodes.length'), 2);
  await evaluate('$("network").value="testnet";$("network").dispatchEvent(new Event("change"));$("input").value="33".repeat(32);read()');
  assert.match(await evaluate('$("status").textContent'), /explorer serves mainnet/);
  await send('Emulation.setDeviceMetricsOverride', { width: 390, height: 844, deviceScaleFactor: 1, mobile: true });
  await navigate(offline);
  assert.equal(await evaluate('document.documentElement.scrollWidth > innerWidth'), false);
  await screenshot('after-mobile-entry');
  await evaluate('$("open-map").click();$("map-demo").click()');
  await wait('mapResult !== null');
  await evaluate('$("protocol-map").scrollIntoView()');
  assert.equal(await evaluate('document.documentElement.scrollWidth > innerWidth'), false);
  assert.ok(await evaluate('$("map-input").getBoundingClientRect().width > 300'), 'map input remains usable on mobile');
  await screenshot('after-mobile-map');
  assert.deepEqual(exceptions, [], 'uncaught browser errors');
  console.log('PASS: reader, context isolation, maps, caps, stale responses, offline/live lookup, all existing surfaces, mobile layout; no uncaught browser errors.');
} finally {
  ws.close();
  chrome.kill('SIGTERM');
  await new Promise(r => chrome.once('exit', r));
  await rm(profile, { recursive: true, force: true });
}
