"use strict";
const test = require("node:test");
const assert = require("node:assert/strict");
const fs = require("node:fs"); const path = require("node:path"); const vm = require("node:vm");
const { parseHTML } = require("linkedom");
const CostSpans = require("../cost-spans.js");
function page() { return parseHTML(fs.readFileSync(path.join(__dirname,"../index.html"),"utf8")).document; }
const span = {offset:12,line:2,col:1,kind:"start"};
function report() { return {mapStatus:"aligned",totalJit:100,attributedJit:30,ambiguousJit:50,unattributedJit:20,exactShare:.3,rows:[
  {label:"GT",rule:"exact",jit:30,share:.3,reason:"one opcode node",candidates:[{irId:1,span}]},
  {label:"ExtractAmount",rule:"ambiguous",jit:50,share:.5,reason:"several opcode nodes",candidates:[{irId:2,span},{irId:3,span:null}]},
  {label:"<img src=x>",rule:"unattributed",jit:20,share:.2,reason:"no node",candidates:[]},
]}; }
test("cost rows keep exact ownership, candidates and unattributed costs separate", () => {
  const root=page().getElementById("cost-spans"); let selected;
  CostSpans.render(report(),root,{onSelect:s=>{selected=s;},partial:true});
  assert.match(root.textContent,/100 JIT units; 30.0% exactly attributed/);
  assert.match(root.textContent,/Exact 30; ambiguous 50; unattributed 20/);
  assert.match(root.textContent,/Synthetic.*nodeValidated: false/);
  assert.match(root.textContent,/trace may be partial/);
  assert.match(root.textContent,/start only/);
  assert.equal(root.querySelectorAll("li").length,3);
  assert.equal(root.querySelector('[data-rule="ambiguous"]').querySelectorAll("button").length,1);
  assert.match(root.querySelector('[data-rule="ambiguous"]').textContent,/Candidate IR 3: no compiler position/);
  root.querySelector("button").click(); assert.deepEqual(selected,span);
  assert.equal(root.querySelectorAll("img,script").length,0);
  CostSpans.render(report(),root); assert.equal(root.querySelectorAll("button").length,0,"no links to an unrelated editor source");
});
test("empty, unavailable and nonaligned traces are explicit and bad totals are refused", () => {
  const root=page().getElementById("cost-spans");
  CostSpans.render(null,root); assert.match(root.textContent,/unavailable/);
  CostSpans.render({...report(),totalJit:1},root); assert.match(root.textContent,/did not reconcile/); assert.equal(root.querySelectorAll("li").length,0);
  CostSpans.render({mapStatus:"misaligned",totalJit:0,attributedJit:0,ambiguousJit:0,unattributedJit:0,exactShare:0,rows:[]},root);
  assert.match(root.textContent,/misaligned/); assert.match(root.textContent,/No cost was recorded/); assert.doesNotMatch(root.textContent,/NaN|Infinity/);
});
test("production Run renders costs from its eval response and rejects stale responses", async () => {
  const app=fs.readFileSync(path.join(__dirname,"../app.js"),"utf8");
  const document=page(); const $=id=>document.getElementById(id); $("scenario").value='{"height":200}'; $("read").hidden=true;
  let finish, requests=0, expectedParam=10, source="sigmaProp(HEIGHT > 100)", selected;
  const context=vm.createContext({ document,$,CostSpans,JSON,editorValue:()=>source,editor:{getValue:()=>source},collectParams:()=>({h:{type:"Int",value:10}}),markValues(){},explainFailure(){},selectInEditor:(off)=>{selected=off;},fetch:async(url,options)=>{
    requests++; assert.equal(url,"/api/v1/eval"); if (JSON.parse(options.body) === null) return {ok:false,json:async()=>({error:{message:"scenario required"}})}; assert.equal(JSON.parse(options.body).source,source); assert.equal(JSON.parse(options.body).params.h.value,expectedParam); return new Promise(resolve=>{finish=resolve;});
  } });
  vm.runInContext('let lastCompiled = {}; let contextGeneration = 0; let diagnosticsGeneration = 0;',context);
  const start=app.indexOf("const EVAL_VERDICTS"); vm.runInContext(app.slice(start,app.indexOf('// ── scenario starting points',start)),context);
  const response={ok:true,json:async()=>({verdict:"pass",cost:10,costLimit:100,reducedTo:"true",trace:[],costSpans:report()})};
  const pending=vm.runInContext("runScenario()",context); await Promise.resolve(); finish(response); await pending;
  assert.equal(requests,1); assert.match($("cost-spans").textContent,/30.0% exactly attributed/);
  $("cost-spans").querySelector("button").click(); assert.equal(selected,12);
  selected=undefined; vm.runInContext('++diagnosticsGeneration',context); $("cost-spans").querySelector("button").click(); assert.equal(selected,undefined);
  $("scenario").value=JSON.stringify({source,height:200,params:{h:{type:"Int",value:500}}}); expectedParam=500;
  const stale=vm.runInContext("runScenario()",context); await Promise.resolve(); vm.runInContext('++diagnosticsGeneration',context); CostSpans.invalidate($("cost-spans")); finish(response); await stale;
  assert.match($("cost-spans").textContent,/Run the current scenario/);
  assert.equal($("run").disabled,false);
  $("scenario").value="null"; await vm.runInContext("runScenario()",context);
  assert.equal($("run").disabled,false); assert.match($("eval-status").textContent,/scenario required/);
});
