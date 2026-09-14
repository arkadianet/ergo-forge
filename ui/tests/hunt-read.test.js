const test = require("node:test");
const assert = require("node:assert/strict");
const { parseHTML } = require("linkedom");
const HuntRead = require("../hunt-read.js");
const ClaimLabels = require("../claim-labels.js");
test("Read keeps rent static and immobilisation scoped and synthetic", () => {
  const { document } = parseHTML('<div id="rent"></div><table><tbody id="probes"></tbody></table><div id="scope"></div><p id="verdict"></p>');
  const elements = Object.fromEntries(["rent", "probes", "scope"].map(id => [id, document.getElementById(id)]));
  const h = {
    verdict: "notUnderProbes", selfSynthetic: false, nodeValidated: false,
    observation: "unspendable-under-probe: not spendable under these probes; every probe errored",
    rentLine: { text: "After four years, anyone may claim this box for the rent. Collection is not predicted.", estimate: { feeNanoerg: 62500000, boxBytes: 50, periodBlocks: 1051200, feeFactor: 1250000, nextCollectionHeight: 1051300 } },
    caps: { maxProbes: 12, maxBoxesPerCollection: 16, blockCostLimit: 8001091, minValuePerByte: 360, basis: "pinned node structural.rs" },
    truncated: true, probeSet: ["registers-absent", "minimum-output-value", "cost-limit"], registerReads: ["SELF.R4[Int].get"],
    probes: [{ kind: "registers-absent", height: 100, output: "preserve", verdict: "error", observation: "unspendable-under-probe", error: "<img src=x onerror=alert(1)>", erroringReads: ["SELF.R4[Int].get"], outputValues: [0], cost: 7, costLimit: 8001091, costExhausted: false }],
  };
  HuntRead.render(h, elements);
  ClaimLabels.renderHunt(h, document.getElementById("verdict"));
  assert.equal(elements.rent.dataset.provenance, "static");
  assert.match(elements.rent.textContent, /static.*After four years.*62500000.*1051300/);
  assert.match(elements.rent.textContent, /P03:storage-rent-acceptance/);
  assert.match(elements.rent.querySelectorAll("a")[1].href, /storage-rent-acceptance.fixture$/);
  assert.match(elements.scope.textContent, /synthetic.*1\/12.*truncated: true.*cost-limit/);
  assert.match(elements.probes.textContent, /SELF.R4\[Int\].get/);
  assert.match(elements.probes.textContent, /7\/8001091; exhausted: false/);
  assert.equal(document.querySelector("img"), null);
  assert.doesNotMatch(document.body?.textContent || document.toString(), /safe/i);
  h.rentLine = null; h.probes = [];
  HuntRead.render(h, elements);
  assert.equal(elements.rent.textContent, "");
  assert.equal(elements.probes.children.length, 0);
});
