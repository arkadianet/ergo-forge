"use strict";
const test = require("node:test");
const assert = require("node:assert/strict");
const fs = require("node:fs");
const path = require("node:path");
const vm = require("node:vm");
const { parseHTML } = require("linkedom");
const root = path.join(__dirname, "../..");
const app = fs.readFileSync(path.join(root, "ui/app.js"), "utf8");

test("Build discovers both binding recipes and renders their authored walkthroughs and questions", async () => {
  const document = parseHTML(fs.readFileSync(path.join(root, "ui/index.html"), "utf8")).document;
  const $ = id => document.getElementById(id);
  const examples = ["pool-bound-swap", "successor-locked-vault"].map(name => {
    const suite = JSON.parse(fs.readFileSync(path.join(root, `examples/contracts/recipes/${name}.test.json`), "utf8"));
    // Backend parses this doc with template_doc; Rust gate checks that path.
    const header = suite.source.match(/\/\*\*([\s\S]*?)\*\//)[1].replace(/^\s*\* ?/gm, "").trim();
    return { id: `recipes/${name}`, group: "recipes", name, source: suite.source,
      doc: { name, description: header.split("@param")[0].trim() },
      params: Object.entries(suite.params).map(([name, v]) => ({ name, typeHint: v.type,
        description: header.match(new RegExp(`@param ${name} (.*)`))[1] })) };
  });
  const context = vm.createContext({ document, $, window: { scrollTo() {} },
    startComposer() {}, fetch: async url => ({ json: async () => url === "/api/v1/examples" ? examples : examples.find(e => url.endsWith(e.id)) }) });
  vm.runInContext(app.slice(app.indexOf("let chainHeight = null"), app.indexOf("/// The wizard's answers")), context);
  await vm.runInContext("loadRecipes()", context);
  const cards = [...$("recipes").querySelectorAll(".recipe:not(.custom)")];
  assert.equal(cards.length, 2);
  for (const [index, card] of cards.entries()) {
    card.click();
    assert.equal($("build-step-2").hidden, false);
    assert.match($("wizard-desc").textContent, /Walkthrough:/);
    assert.match($("wizard-desc").textContent, /must.*fail/);
    assert.match($("wizard-desc").textContent, /synthetic/);
    assert.match($("wizard-title").textContent, /Pool-bound swap|Successor-locked vault/);
    for (const p of examples[index].params) {
      assert.ok($(`f-${p.name}`));
      assert.match($(`f-${p.name}`).parentNode.textContent, /Which|Minimum/);
    }
  }
});
