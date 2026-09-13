"use strict";
const test = require("node:test");
const assert = require("node:assert/strict");
const fs = require("node:fs");
const path = require("node:path");
const vm = require("node:vm");
const { parseHTML } = require("linkedom");
const PlayExport = require("../play-export.js");
const app = fs.readFileSync(path.join(__dirname, "../app.js"), "utf8");
function page() { return parseHTML(fs.readFileSync(path.join(__dirname, "../index.html"), "utf8")).document; }
function request() { return {height:200,network:"testnet",boxes:[{boxId:"aa".repeat(32),ergoTree:"10010101d17300",value:7}],tx:{inputs:[{boxId:"aa".repeat(32),contextVars:{0:{type:"Int",value:5}},secrets:[{dlog:"test key"}]}],dataInputs:[],outputs:[]}}; }
const result = {inputs:[{verdict:"proofAccepted"}],nodeValidated:false};
const documentValue = {tree:"10010101d17300",synthetic:true,nodeValidated:false};

test("export buttons send the original Play snapshot and input index, and download the requested document", async () => {
  const root = page().getElementById("play-export");
  const req = request(), original = structuredClone(req), saved = [];
  PlayExport.render(req, {...result,inputs:[...result.inputs,{verdict:"<img src=x>"}]}, root, {
    request: async (url, options) => {
      assert.equal(url,"/api/v1/play/export");
      const {inputIndex,kind,...body} = JSON.parse(options.body);
      assert.deepEqual(body,original);
      assert.equal(inputIndex,1);
      return {ok:true,json:async()=>({...documentValue,kind})};
    },
    download: (doc, name) => saved.push([doc,name]),
  });
  req.height=999; req.boxes[0].spent=true; req.tx.inputs=[];
  assert.equal(root.querySelectorAll("img").length,0);
  assert.match(root.textContent,/Synthetic/); assert.match(root.textContent,/signature-less/);
  const buttons=root.querySelectorAll("button");
  await buttons[2].onclick(); await buttons[3].onclick();
  assert.deepEqual(saved.map(s=>s[1]),["contract.test.json","scenario.json"]);
  assert.deepEqual(saved.map(s=>s[0].kind),["test","scenario"]);
  assert.match(root.textContent,/ergo-es eval scenario.json/);
});

test("failed, falsely labelled and stale exports never download", async () => {
  const root = page().getElementById("play-export");
  let downloads=0;
  for (const response of [{ok:false,json:async()=>({error:{message:"out of range"}})}, {ok:true,json:async()=>({nodeValidated:true})}]) {
    PlayExport.render(request(),result,root,{request:async()=>response,download:()=>downloads++});
    await root.querySelector("button").onclick();
    assert.equal(downloads,0); assert.match(root.textContent,/Export unavailable/);
  }
  let finish;
  PlayExport.render(request(),result,root,{request:()=>new Promise(resolve=>{finish=resolve;}),download:()=>downloads++});
  const pending=root.querySelector("button").onclick();
  PlayExport.render(request(),result,root);
  finish({ok:true,json:async()=>documentValue}); await pending;
  assert.equal(downloads,0);
});

test("download produces plain JSON with the filename and releases the object URL", async () => {
  const document=page(); let blob, revoked, clicked, release;
  const context=vm.createContext({Blob,document,setTimeout:f=>{release=f;},URL:{createObjectURL:b=>{blob=b;return "blob:export";},revokeObjectURL:u=>{revoked=u;}}});
  vm.runInContext(fs.readFileSync(path.join(__dirname,"../play-export.js"),"utf8"),context);
  document.addEventListener("click",e=>{clicked={download:e.target.download,href:e.target.href};});
  vm.runInContext('PlayExport.download(document, {synthetic:true,nodeValidated:false}, "scenario.json")',context);
  assert.deepEqual(JSON.parse(await blob.text()),{synthetic:true,nodeValidated:false});
  assert.deepEqual(clicked,{download:"scenario.json",href:"blob:export"});
  release(); assert.equal(revoked,"blob:export");
});

test("production Play handler exports successful and refused results after the input boxes change", async () => {
  for (const accepted of [true,false]) {
    const document=page(), $=id=>document.getElementById(id), req=request();
    $("play").hidden=false;
    const input=document.createElement("div");input.dataset.boxId=req.boxes[0].boxId;
    input.innerHTML='<textarea class="tx-secrets"></textarea><textarea class="tx-vars">{}</textarea>';
    $("tx-inputs").appendChild(input);
    let sent, exported;
    const context=vm.createContext({document,$,PlayExport:{render:(request,body,root)=>{exported=structuredClone(request);PlayExport.render(request,body,root);}},
      playNetwork:()=>"testnet",readOutputs(){},nanoOf:Number,parseTokens:()=>[],renderPlay:async()=>{$("play-tx").hidden=true;},savePlay(){},
      fetch:async(url,options)=>{sent=JSON.parse(options.body);return {ok:true,json:async()=>({ok:accepted,inputs:[{boxId:req.boxes[0].boxId,verdict:accepted?"pass":"fail"}],outputs:[],problems:[],txId:"ff".repeat(32)})};}});
    context.initialPlay={height:200,boxes:req.boxes,history:[]};
    vm.runInContext('let play=initialPlay;let txOutputs=[];const selectedBoxes=new Set();',context);
    // Capture the real click handler so this assertion awaits its completion.
    let handler; $("tx-send").addEventListener=(name,fn)=>{handler=fn;};
    const start=app.indexOf('const ANYONE_TREE =');
    vm.runInContext(app.slice(start,app.indexOf('// Build → Play:',start)),context);
    await handler();
    assert.deepEqual(exported,sent);
    assert.equal(exported.boxes[0].spent,undefined);
    assert.equal($("tx-result").hidden,false);
    assert.ok(!$("play-tx").contains($("tx-result")),"successful export controls remain outside the hidden transaction form");
    assert.equal($("play-export").querySelectorAll("button").length,2);
  }
});
