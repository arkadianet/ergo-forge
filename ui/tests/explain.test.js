"use strict";
const test = require("node:test"); const assert = require("node:assert/strict");
const fs=require("node:fs"); const path=require("node:path"); const vm=require("node:vm");
const {parseHTML}=require("linkedom"); const Explain=require("../explain.js"); const CostSpans=require("../cost-spans.js");
function page() {return parseHTML(fs.readFileSync(path.join(__dirname,"../index.html"),"utf8")).document;}
function report() {return {message:"values recorded by this synthetic sandbox reduction",mapStatus:"aligned",selectionRule:"first-start-in-selection-then-innermost",expression:{irId:2,opcode:"BoolToSigmaProp (0xD1)",span:{offset:10,line:1,col:11,kind:"start"},values:[{value:"SigmaProp(TrivialProp(false))",residual:"false",truncated:false},{value:"<img src=x>...",truncated:true,residualNote:"No complete residual was inferred"}],residual:null}};}
test("selection converts UTF-16 indices to exact UTF-8 bytes and refuses empty or split selections",()=>{
  const source="// 😀\r\nsigmaProp(HEIGHT > 1)"; const i=source.indexOf("HEIGHT");
  assert.deepEqual(Explain.selection(source,i,i+6),{offset:Buffer.byteLength(source.slice(0,i)),length:6});
  for(const range of [[0,0],[4,5],[-1,6],[0,100]]) assert.throws(()=>Explain.selection(source,...range));
  const payload=Explain.payload({height:123,contextVars:{1:{type:"Int",value:9}}},source,{h:{type:"Int",value:10}},"testnet",Explain.selection(source,i,i+6));
  assert.equal(payload.scenario.source,source); assert.equal(payload.scenario.height,123); assert.equal(payload.scenario.contextVars[1].value,9); assert.equal(payload.scenario.network,"testnet");
  assert.equal(Explain.payload({source,params:{h:{type:"Int",value:500}},network:"mainnet"},source,{h:{type:"Int",value:10}},"testnet",{}).scenario.params.h.value,500);
  for(const sc of [[],null,{tree:"abc"},{source:"other"}]) assert.throws(()=>Explain.payload(sc,source,{},"mainnet",{}));
});
test("explanation renders values, residuals, no-match and truncated results as text",()=>{
  const root=page().getElementById("explain-result"); let clicked;
  Explain.render(report(),root,{onSelect:span=>{clicked=span;}});
  assert.match(root.textContent,/nodeValidated: false/); assert.match(root.textContent,/Residual: false/);
  assert.match(root.textContent,/engine truncated/); assert.match(root.textContent,/No complete residual/);
  root.querySelector("button").click(); assert.equal(clicked.offset,10);
  assert.equal(root.querySelectorAll("img,script").length,0);
  Explain.render({...report(),expression:null,message:"no evaluated expression starts in this selection"},root);
  assert.match(root.textContent,/no evaluated expression starts/); assert.equal(root.querySelectorAll("button,pre").length,0);
});
test("explain load uses one full scenario request and rejects stale responses and failures",async()=>{
  const root=page().getElementById("explain-result"); let finish; let requests=0;
  const request=async(url,options)=>{requests++;assert.equal(url,"/api/v1/explain");assert.equal(JSON.parse(options.body).scenario.height,10);return new Promise(resolve=>{finish=resolve;});};
  const pending=Explain.load({scenario:{height:10}},root,{request});
  Explain.invalidate(root,"Source changed"); finish({ok:true,json:async()=>report()}); await pending;
  assert.equal(requests,1); assert.equal(root.textContent,"Source changed");
  await Explain.load({},root,{request:async()=>({ok:false,json:async()=>({error:{message:"missing parameters"}})})});
  assert.match(root.textContent,/Explanation unavailable: missing parameters/);
});
test("production Explain sends the selected editor snapshot and invalidates on edits",async()=>{
  const app=fs.readFileSync(path.join(__dirname,"../app.js"),"utf8"); const document=page(); const $=id=>document.getElementById(id);
  $("scenario").value='{"height":234,"selfBox":{"value":500}}'; $("write-network").querySelector('[value="testnet"]').selected=true;
  const source="// 😀\nsigmaProp(HEIGHT > 1)"; const from=source.indexOf("HEIGHT"); const listeners={}; let payload,options;
  const context=vm.createContext({document,$,CostSpans,Explain:{...Explain,load:async(p,root,o)=>{payload=p;options=o;Explain.render(report(),root,o);}},editorValue:()=>source,collectParams:()=>({limit:{type:"Int",value:10}}),editor:{on:(e,f)=>{listeners[e]=f;},listSelections:()=>[{}],getCursor:edge=>edge==="from"?from:from+6,indexFromPos:i=>i},markValues(){},selectInEditor(){}});
  vm.runInContext("let contextGeneration=0;",context);
  const start=app.indexOf("// ── Write source diagnostics"); vm.runInContext(app.slice(start,app.indexOf("// ── scenario eval",start)),context);
  await vm.runInContext("explainSelection()",context);
  assert.equal(payload.scenario.source,source); assert.equal(payload.scenario.height,234); assert.equal(payload.scenario.selfBox.value,500); assert.equal(payload.scenario.params.limit.value,10); assert.equal(payload.scenario.network,"testnet");
  assert.equal(payload.selection.offset,Buffer.byteLength(source.slice(0,from))); assert.equal(options.isCurrent(),true);
  listeners.change(); assert.equal(options.isCurrent(),false); assert.match($("explain-result").textContent,/Select an expression/);
  for(const id of ["scenario","params-rows","write-network"]) {Explain.render(report(),$("explain-result"));$(id).dispatchEvent(new document.defaultView.Event("input"));assert.equal($("explain-result").querySelectorAll("pre").length,0);}
});
