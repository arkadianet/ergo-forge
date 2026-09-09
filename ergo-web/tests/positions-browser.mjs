// Browser regression checks using system Chromium directly; no npm dependencies.
// node ergo-web/tests/positions-browser.mjs [URL] [--before]
// --before captures the three baseline views against the unchanged shell.
import { spawn } from 'node:child_process';
import { mkdtemp, mkdir, writeFile, readFile, rm } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join, resolve } from 'node:path';
import assert from 'node:assert/strict';

const offline = process.argv[2] || 'http://127.0.0.1:8099';
const shots = resolve('ui/positions-review/screenshots');
const before = process.argv.includes('--before');
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
  await send('Page.navigate', { url: 'about:blank' });
  await wait('typeof configReady === "undefined"');
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
  await send('Emulation.setDeviceMetricsOverride', { width: 1440, height: 1100, deviceScaleFactor: 1, mobile: false });
  await navigate(offline + '/#write');
  await evaluate(String.raw`editor.setValue("{\n  val limit = 1000\n  sigmaProp(HEIGHT > true)\n}"); compile()`);
  await screenshot(before ? 'before-type' : 'after-type');
  await evaluate('editor.setValue("HEIGHT");compile()');
  await screenshot(before ? 'before-root' : 'after-root');
  await evaluate('(async()=>{editor.setValue("sigmaProp(HEIGHT > 1000)");await compile();showTab("scenario");$("scenario").value=JSON.stringify({height:1001});await runScenario();})()');
  if (!before) await evaluate('window.scrollTo(0,150)');
  await screenshot(before ? 'before-cost' : 'after-cost');
  await evaluate('window.scrollTo(0,0)');

  if (!before) {
    assert.ok(await evaluate('$("cost-hot-spots").children.length > 0'));
    assert.match(await evaluate('$("cost-hot-spots-summary").textContent'), /JIT units recorded/);
    // Exact starts: multiline Unicode, tabs, end of input, genuine zero.
    for (const [source, phase, expected] of [
      ['// café 🦀\n\tsigmaProp(HEIGHT > true)', 'Type', { line: 1, ch: 11 }],
      ['sigmaProp(', 'Parse', { line: 0, ch: 10 }],
      ['@', 'Parse', { line: 0, ch: 0 }],
      ['PK("not-an-address")', 'Bind', { line: 0, ch: 0 }],
    ]) {
      await evaluate(`editor.setValue(${JSON.stringify(source)});editor.setCursor({line:0,ch:1});compile()`);
      assert.match(await evaluate('$("compile-status").textContent'), new RegExp(`^${phase} error:`));
      assert.deepEqual(await evaluate('({line:editor.getCursor().line,ch:editor.getCursor().ch})'), expected);
      assert.equal(await evaluate('editor.somethingSelected()'), false);
      assert.equal(await evaluate('editor.hasFocus()'), true);
      assert.deepEqual(await evaluate('({line:compileErrorMark.find().line,ch:compileErrorMark.find().ch})'), expected);
      assert.equal(await evaluate('compileErrorMark.type'), 'bookmark', 'point, not a guessed range');
      assert.equal(await evaluate('document.querySelectorAll(".cm-error-point").length'), 1);
      if (source.startsWith('//')) await screenshot('after-unicode');
      if (source === 'sigmaProp(') await screenshot('after-eof');
      await evaluate('editor.replaceRange(" ",editor.getCursor())');
      assert.equal(await evaluate('compileErrorMark'), null);
      assert.equal(await evaluate('$("caret").hidden'), true);
      assert.equal(await evaluate('$("compile-status").hidden'), true);
    }
    for (const [source, phase] of [
      ['HEIGHT', 'Root'],
      ['sigmaProp((HEIGHT & 1) == 0)', 'Emit'],
      ['unsignedBigInt("5") > unsignedBigInt("3")', 'Serializer'],
    ]) {
      await evaluate(`editor.setValue(${JSON.stringify(source)});editor.setCursor({line:0,ch:3});compile()`);
      assert.match(await evaluate('$("compile-status").textContent'), new RegExp(`^${phase} error:.*No source position`));
      assert.equal(await evaluate('compileErrorMark'), null);
      assert.equal(await evaluate('$("caret").hidden'), true);
      assert.deepEqual(await evaluate('({line:editor.getCursor().line,ch:editor.getCursor().ch})'), { line: 0, ch: 3 });
    }
    // Write is not naturally reachable via a valid source: exercise its DTO
    // presentation explicitly, with HTTP serialization covered in Rust.
    await evaluate('showCompileError({code:"compile_error",phase:"write",offset:0,offsetSource:"unavailable",message:"Fixture: serialization failed"})');
    assert.match(await evaluate('$("compile-status").textContent'), /^Write error:.*No source position/);
    assert.equal(await evaluate('compileErrorMark'), null);
    await evaluate(String.raw`editor.setValue('{ val bytes = fromBase16("$data"); sigmaProp(HEIGHT > true) }');renderParams([{name:'data',typeHint:'Coll[Byte]',default:'abcdef0123456789'}]);compile()`);
    assert.match(await evaluate('$("compile-status").textContent'), /after string parameter substitution/);
    assert.equal(await evaluate('compileErrorMark'), null);
    await screenshot('after-substitution');
    await evaluate('document.querySelector("#params-rows input").dispatchEvent(new Event("input",{bubbles:true}))');
    assert.equal(await evaluate('$("compile-status").hidden'), true);
    // A late error must not move the caret or mark edited/replaced source.
    await evaluate(`(async()=>{editor.setValue('sigmaProp(HEIGHT > true)');const real=window.fetch;window.fetch=async(...args)=>{const response=await real(...args);if(String(args[0]).endsWith('/compile')) await new Promise(r=>setTimeout(r,150));return response;};try {const run=compile();editor.setValue('sigmaProp(true)');await run;}finally{window.fetch=real;}})()`);
    assert.equal(await evaluate('compileErrorMark'), null);
    assert.equal(await evaluate('$("compile-status").hidden'), true);
    // Changing compilation inputs or mode also invalidates an in-flight error.
    for (const change of [
      '$("write-network").value="testnet";$("write-network").dispatchEvent(new Event("change"))',
      'document.querySelector("#params-rows input").dispatchEvent(new Event("input",{bubbles:true}))',
      'setMode("read")',
    ]) {
      await evaluate(`(async()=>{setMode('write');editor.setValue('sigmaProp(HEIGHT > true)');renderParams([{name:'limit',typeHint:'Int',default:'1'}]);const real=window.fetch;window.fetch=async(...args)=>{const response=await real(...args);if(String(args[0]).endsWith('/compile'))await new Promise(r=>setTimeout(r,150));return response;};try{const run=compile();${change};await run;}finally{window.fetch=real;}})()`);
      assert.equal(await evaluate('compileErrorMark'), null);
      assert.equal(await evaluate('$("compile-status").hidden'), true);
    }
    await evaluate('setMode("write");editor.setValue("sigmaProp(true)");renderParams([])');
    // Valid compilation clears a previous error.
    await evaluate('compile()');
    assert.equal(await evaluate('$("compiled").hidden'), false);
    assert.equal(await evaluate('$("caret").hidden'), true);
    // Run the reader's recovered tree and check runtime errors retain their
    // own ranked trace, then a passing run replaces it.
    await evaluate('(async()=>{setMode("read");$("input").value="1001040ad191e4c6a704047300";await read();$("read-test").click();$("scenario").value=JSON.stringify({height:1001});await runScenario();})()');
    assert.match(await evaluate('$("eval-verdict").textContent'), /^ERROR/);
    assert.ok(await evaluate('$("cost-hot-spots").children.length > 0'));
    assert.match(await evaluate('$("cost-hot-spots-summary").textContent'), /trace may be partial/);
    await evaluate('$("eval-result").scrollIntoView()');
    await screenshot('after-reader-cost');
    await evaluate('(async()=>{setMode("write");editor.setValue("sigmaProp(HEIGHT > 1000)");await compile();showTab("scenario");$("scenario").value=JSON.stringify({height:1000});await runScenario();})()');
    assert.match(await evaluate('$("eval-verdict").textContent'), /^FAIL/);
    assert.doesNotMatch(await evaluate('$("cost-hot-spots-summary").textContent'), /partial/);
    // Budget exhaustion is a normal result, with only recorded steps shown.
    await evaluate('$("scenario").value=JSON.stringify({height:1001,costLimit:1});runScenario()');
    assert.match(await evaluate('$("eval-verdict").textContent'), /^ERROR/);
    assert.match(await evaluate('$("cost-hot-spots-summary").textContent'), /partial|No cost steps/);
    // Empty traces have an honest empty state (presentation fixture).
    await evaluate('renderHotSpots([],"pass")');
    assert.equal(await evaluate('$("cost-hot-spots-table").hidden'), true);
    assert.match(await evaluate('$("cost-hot-spots-summary").textContent'), /No cost steps/);
    // Legacy routes and a source+parameter+network share link survive.
    for (const mode of ['build', 'write', 'play']) {
      await navigate(offline + '/#' + mode);
      assert.equal(await evaluate('document.querySelector(".mode.active").textContent.toLowerCase()'), mode);
    }
    await evaluate('setMode("write");editor.setValue("sigmaProp(HEIGHT > $limit)");renderParams([{name:"limit",typeHint:"Int",default:"1000"}]);$("write-network").value="testnet"');
    const share = await evaluate('encodeShare()');
    await navigate(offline + '/#s=' + share);
    await wait('editor.getValue().includes("$limit")');
    assert.equal(await evaluate('$("write-network").value'), 'testnet');
    assert.deepEqual(await evaluate('collectParams()'), {limit:{type:'Int',value:1000}});
    await evaluate('compile()');
    assert.equal(await evaluate('$("compiled").hidden'), false);
    await evaluate('(async()=>{showTab("tests");$("tests").value=JSON.stringify([{name:"before",expect:"fail",height:1000},{name:"after",expect:"pass",height:1001}]);await runTests();})()');
    assert.equal(await evaluate('$("tests-summary").textContent'), '2 passed, 0 failed');
    // Play's existing fund/select/send loop.
    await evaluate('setMode("play");$("play-fund-toggle").click();$("fund-contract").value="10010101d17300";$("fund-go").click()');
    await wait('document.querySelector("#play-boxes button")');
    await evaluate('document.querySelector("#play-boxes button").click()');
    await wait('!$("play-tx").hidden && document.querySelector(".o-dest")');
    await evaluate('$("tx-send").click()');
    await wait('$("tx-status").textContent.startsWith("Accepted")');
    // Mobile positioning with wrapping and an off-screen source location.
    await send('Emulation.setDeviceMetricsOverride', { width: 390, height: 844, deviceScaleFactor: 1, mobile: true });
    await navigate(offline + '/#write');
    await evaluate('editor.setValue("// " + "long comment ".repeat(20) + "\\n" + "\\n".repeat(45) + "sigmaProp(HEIGHT > true)");compile()');
    assert.equal(await evaluate('document.documentElement.scrollWidth > innerWidth'), false);
    assert.equal(await evaluate('editor.getCursor().line'), 46);
    assert.ok(await evaluate('editor.getScrollInfo().top > 0 || window.scrollY > 0'));
    assert.ok(await evaluate('editor.charCoords(editor.getCursor(),"window").top < innerHeight'));
    await screenshot('after-mobile-scroll');
    await evaluate(String.raw`editor.setValue("// café 🦀\n{\n  val limit = 1000\n  sigmaProp(HEIGHT > true)\n}");compile();window.scrollTo(0,0)`);
    await screenshot('after-mobile-position');
    await evaluate('(async()=>{editor.setValue("sigmaProp(HEIGHT > 1000)");await compile();showTab("scenario");$("scenario").value=JSON.stringify({height:1001});await runScenario();$("cost-hot-spots-title").scrollIntoView();})()');
    assert.equal(await evaluate('document.documentElement.scrollWidth > innerWidth'), false);
    await screenshot('after-mobile-cost');
  }
  assert.deepEqual(exceptions, [], 'uncaught browser errors');
  console.log(before ? 'PASS: baseline screenshots' : 'PASS: compiler phases, Unicode/EOF/zero carets, stale errors, substitutions, ranked costs, reader, suites, legacy routes, shared source, Play, mobile; no uncaught browser errors.');
} finally {
  ws.close();
  chrome.kill('SIGTERM');
  await new Promise(r => chrome.once('exit', r));
  await rm(profile, { recursive: true, force: true });
}
