"use strict";
// Read's bounded observations. All response text is inserted as text.
const HuntRead = (() => {
  function add(root, tag, text, className) {
    const el = root.ownerDocument.createElement(tag);
    el.textContent = text;
    if (className) el.className = className;
    root.appendChild(el);
    return el;
  }
  function render(h, elements) {
    const { rent, probes, scope } = elements;
    rent.replaceChildren();
    if (h.rentLine) {
      rent.dataset.provenance = "static";
      add(rent, "span", "static", "chip");
      add(rent, "span", ` ${h.rentLine.text} `);
      const r = h.rentLine.estimate;
      add(rent, "span", `Estimated ${r.feeNanoerg} nanoERG for ${r.boxBytes} bytes every ${r.periodBlocks} blocks (${r.feeFactor} nanoERG/byte).${r.nextCollectionHeight == null ? " Creation height unknown." : ` Next estimated eligibility height: ${r.nextCollectionHeight}.`} `);
      const base = "https://github.com/arkadianet/ergo-forge/blob/main/";
      for (const [label, path] of [
        ["STORAGE_PERIOD / STORAGE_FEE_FACTOR (frozen)", "ergo-sandbox/src/rent.rs"],
        ["P03:storage-rent-acceptance (node-validated basis)", "ergo-sandbox/tests/fixtures/evidence/node-vectors/storage-rent-acceptance.fixture"],
      ]) {
        add(rent, "a", label).href = base + path;
        add(rent, "span", " ");
      }
    }
    scope.replaceChildren();
    add(scope, "span", "synthetic", "chip");
    const c = h.caps;
    add(scope, "span", ` ${h.observation || "Not spendable under these probes"}. ${h.probes.length}/${c.maxProbes} probes; box cap ${c.maxBoxesPerCollection} per collection; truncated: ${h.truncated}. Set: ${(h.probeSet || []).join(", ")}. Block budget ${c.blockCostLimit}; minimum value factor ${c.minValuePerByte} nanoERG/byte (pinned defaults). Full node validation has not run.`);
    add(scope, "p", `Rule basis: ${c.basis}`);
    if (h.registerReads?.length) add(scope, "p", `Register reads in recovered code: ${h.registerReads.join(", ")}`);
    probes.replaceChildren();
    for (const p of h.probes) {
      const tr = add(probes, "tr", "");
      tr.dataset.verdict = p.verdict;
      const detail = [p.observation, p.error ? `error: ${p.error}` : p.reducedTo,
        p.erroringReads?.length ? `Absent .get operands reached before failure: ${p.erroringReads.join(", ")}` : "",
        `Outputs: ${(p.outputValues || []).join(", ")} nanoERG`].filter(Boolean).join("; ");
      for (const text of [p.kind, p.height, p.output, p.verdict, `${p.cost}/${p.costLimit}; exhausted: ${p.costExhausted}`, detail]) add(tr, "td", String(text));
    }
  }
  return { render };
})();
if (typeof module !== "undefined") module.exports = HuntRead;
