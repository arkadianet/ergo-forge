"use strict";
const test = require("node:test");
const assert = require("node:assert/strict");
const fs = require("node:fs");
const path = require("node:path");
const vm = require("node:vm");
const { parseHTML } = require("linkedom");
const Share = require("../share.js");
const app = fs.readFileSync(path.join(__dirname,"../app.js"),"utf8");
function page() { return parseHTML(fs.readFileSync(path.join(__dirname,"../index.html"),"utf8")).document; }
function playState() { return {k:"play",v:1,height:200,network:"testnet",boxes:[{boxId:"aa".repeat(32),ergoTree:"10010101d17300",value:7,spent:false,tokens:[],registers:{}}],history:["λ → funded"],synthetic:true,nodeValidated:false}; }
const suite={tree:"10010101d17300",network:"testnet",scenarios:[{name:"Synthetic λ",expect:"pass",height:200}]};
const raw = value=>Buffer.from(JSON.stringify(value)).toString("base64url");

test("legacy Write, Play with optional history, and full suites round-trip offline", () => {
  for (const state of [{s:"// λ 🌋\nsigmaProp(true)",p:{h:{type:"Int",value:1}},n:"testnet"},{s:""},playState(),{...playState(),history:undefined},{k:"suite",v:1,suite}]) {
    const expected=JSON.parse(JSON.stringify(state));
    assert.equal(Share.encodeShare(state),raw(state));
    assert.deepEqual(Share.decodeShare(Share.encodeShare(state)),expected);
    assert.deepEqual(Share.decodeShare("#s="+Share.encodeShare(state)),expected);
  }
});

test("both cap refusals give actual encoded bytes and cap, including unicode and #s=", () => {
  const state={s:"λ".repeat(Share.FRAGMENT_CAP)}, fragment=raw(state), size=fragment.length+3;
  const check=e=>e.message.includes(`${size} bytes`)&&e.message.includes(`${Share.FRAGMENT_CAP} bytes`);
  assert.throws(()=>Share.encodeShare(state),check);
  assert.throws(()=>Share.decodeShare(fragment),check);
  assert.throws(()=>Share.decodeShare("#s="+fragment),check);
});

test("malformed payloads are rejected before use", () => {
  for (const value of [null,[],{}, {k:"other",v:1}, {...playState(),v:2}, {...playState(),nodeValidated:true}, {...playState(),height:-1}, {...playState(),network:"other"}, {...playState(),history:{}}, {...playState(),boxes:[{...playState().boxes[0],boxId:'<img src=x>'}]}, {k:"suite",v:1,suite:{...suite,source:"sigmaProp(true)"}}, {k:"suite",v:1,suite:{...suite,scenarios:[{...suite.scenarios[0],tree:suite.tree}]}}, {k:"suite",v:1,suite:{...suite,scenarios:[{...suite.scenarios[0],expect:"safe"}]}}]) {
    assert.throws(()=>Share.encodeShare(value)); assert.throws(()=>Share.decodeShare(raw(value)));
  }
  for (const fragment of ["", "!", "a", "e30=", "_w", "not-json"]) assert.throws(()=>Share.decodeShare(fragment));
});

test("Play requires an explicit confirm control; cancel and loading never replace or save", () => {
  const root=page().getElementById("share-confirm");let replacements=0, received;
  const state=playState();
  Share.confirmPlay(state,root,value=>{replacements++;received=value;});
  assert.equal(replacements,0);assert.match(root.textContent,/synthetic/);assert.match(root.textContent,/Not node-validated/);
  root.querySelectorAll("button")[1].click(); assert.equal(replacements,0);assert.equal(root.hidden,true);
  Share.confirmPlay(state,root,value=>{replacements++;received=value;});
  state.height=999; const button=root.querySelector("button");button.click();button.click();
  assert.equal(replacements,1);assert.equal(received.height,200);
});

function appContext() {
  const document=page(), $=id=>document.getElementById(id);let saves=0, renders=0; const modes=[],notices=[],copies=[];
  const context=vm.createContext({document,$,Share,location:{origin:"https://forge.example",pathname:"/",hash:""},history:{replaceState:(a,b,c)=>notices.push(c)},
    setMode:(...args)=>modes.push(args),showTab:tab=>notices.push(tab),setEditorValue:source=>notices.push(source),renderParams:params=>notices.push(params),editorValue:()=>"sigmaProp(true)",collectParams:()=>({}),copyText:(...args)=>copies.push(args),
    savePlay:()=>saves++,renderPlay:inspect=>{assert.equal(inspect,false);renders++;},fetch:()=>{throw Error("share must not fetch");}});
  vm.runInContext('let play={height:1,boxes:[],history:[],words:{}};const selectedBoxes=new Set(["old"]);let txOutputs=[];',context);
  const start=app.indexOf('/// Codecs live in share.js;');
  vm.runInContext(app.slice(start,app.indexOf('async function copyText',start)),context);
  const current=app.indexOf('function currentSuite()');
  vm.runInContext(app.slice(current,app.indexOf('async function runTests()',current)),context);
  return {context,$,document,modes,notices,copies,saves:()=>saves,renders:()=>renders};
}

test("production loader preserves storage until confirmation and loads a suite intact into Tests", async () => {
  const c=appContext();c.context.fragment=Share.encodeShare(playState());
  await vm.runInContext('loadShared(fragment)',c.context);
  assert.equal(c.saves(),0);assert.equal(c.renders(),0);
  assert.equal(c.modes[0][0],"play");assert.equal(c.modes[0][1].inspectPlay,false);
  c.$("share-confirm").querySelector("button").click();assert.equal(c.saves(),1);assert.equal(c.renders(),1);
  assert.equal(vm.runInContext('play.network',c.context),"testnet");
  assert.equal(vm.runInContext('play.nodeValidated',c.context),false);
  assert.equal(vm.runInContext('selectedBoxes.size',c.context),0);
  c.context.fragment=Share.encodeShare({k:"suite",v:1,suite});
  await vm.runInContext('loadShared(fragment)',c.context);
  assert.deepEqual(JSON.parse(c.$("tests").value),suite);
  assert.deepEqual(JSON.parse(JSON.stringify(vm.runInContext('currentSuite().suite',c.context))),suite);
  assert.ok(c.notices.includes("tests"));assert.match(c.$("share-status").textContent,/Synthetic|synthetic/);
  assert.equal(c.saves(),1);
  c.context.fragment=Share.encodeShare({s:"original Write",p:{h:{type:"Int",value:5}},n:"testnet"});
  await vm.runInContext('loadShared(fragment)',c.context);assert.ok(c.notices.includes("original Write"));
  c.context.fragment="a".repeat(Share.FRAGMENT_CAP);
  await vm.runInContext('loadShared(fragment)',c.context);assert.match(c.$("share-status").textContent,/7171 bytes.*7168 bytes/);
  assert.equal(c.saves(),1);
});

test("production copy controls use codecs for all shapes and show size failures without copying", () => {
  const c=appContext();
  c.context.window={addEventListener(){}};c.context.playNetwork=()=>"testnet";
  const start=app.indexOf('$("share").addEventListener');
  vm.runInContext(app.slice(start,app.indexOf('/// Fleet SDK:',start)),c.context);
  c.$("share").click(); c.$("play-share").click(); c.$("tests").value=JSON.stringify(suite);c.$("share-tests").click();
  const states=c.copies.map(([url])=>Share.decodeShare(url.split("#s=")[1]));
  assert.equal(states[0].s,"sigmaProp(true)");assert.equal(states[1].k,"play");assert.deepEqual(states[2].suite,suite);
  c.context.large={s:"x".repeat(Share.FRAGMENT_CAP)};vm.runInContext('copyShare(large)',c.context);
  assert.equal(c.copies.length,3);assert.match(c.$("share-status").textContent,/bytes; cap is 7168 bytes/);
});


test("shared chain rendering stays offline, shows its network, and treats register and history text literally", async () => {
  const c=appContext();
  const state=playState();state.boxes[0].registers={R4:{type:"Coll[Byte]",value:"<img src=x onerror=bad()>"}};
  state.history=["<script>bad()</script>"];
  c.context.imported=state;
  c.context.ergOf=n=>`${n} nanoERG`;c.context.shortAddr=x=>x;
  c.context.renderTxForm=()=>{};
  const words=app.indexOf("async function wordsFor(");
  vm.runInContext(app.slice(words,app.indexOf("/// Resolve what the user typed",words)),c.context);
  const render=app.indexOf("async function renderPlay(");
  vm.runInContext(app.slice(render,app.indexOf("let txOutputs =",render)),c.context);
  vm.runInContext("replaceSharedPlay(imported)",c.context);
  await vm.runInContext("renderPlay()",c.context);
  assert.equal(c.$("play-network").textContent,"testnet");
  assert.match(c.$("play-boxes").textContent,/<img src=x/);
  assert.equal(c.$("play-boxes").querySelectorAll("img,script").length,0);
  assert.match(c.$("play-history").textContent,/<script>/);
  assert.equal(c.$("play-history").querySelectorAll("script").length,0);
  assert.throws(()=>Share.encodeShare({...state,boxes:[{...state.boxes[0],registers:{R4:null}}]}),/typed values/);
  assert.throws(()=>Share.encodeShare({k:"suite",v:1,suite:{...suite,nodeValidated:true}}),/not node-validated/);
});

test("clipboard refusal leaves the share URL visible in the active page", async () => {
  const c=appContext();c.context.navigator={clipboard:{writeText:async()=>{throw Error("denied");}}};
  c.context.toast=()=>{};
  const start=app.indexOf("async function copyText(");
  vm.runInContext(app.slice(start,app.indexOf("/// A brief confirmation",start)),c.context);
  await vm.runInContext('copyText("https://forge.example/#s=payload", "copied", $("share-status"))',c.context);
  assert.equal(c.$("share-status").hidden,false);
  assert.equal(c.$("share-status").textContent,"https://forge.example/#s=payload");
});
