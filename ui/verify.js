"use strict";
// Byte/structural observations only. Server and user data are inserted as text.
const Verify = (() => {
  const requests = new WeakMap();
  function element(root, tag, text) {
    const el = root.ownerDocument.createElement(tag);
    if (text != null) el.textContent = text;
    root.appendChild(el);
    return el;
  }
  function invalidate(root, message = "Verify the current source and parameters against an address or tree hex.") {
    requests.get(root)?.abort();
    requests.delete(root);
    delete root.dataset.outcome;
    root.replaceChildren();
    element(root, "p", message);
  }
  function render(report, root) {
    const labels = { exact: "Exact — identical ErgoTree bytes", template: "Template — same structure with differing constants", no_match: "No match" };
    if (!Object.hasOwn(labels, report.outcome) || !report.limitation) throw new Error("Invalid verification response");
    root.replaceChildren();
    root.hidden = false;
    root.dataset.outcome = report.outcome;
    element(root, "strong", labels[report.outcome]);
    if (report.outcome === "template") {
      element(root, "p", "Left: compiled source. Right: supplied target. Paths are ordered child indices from the root.");
      const list = element(root, "ul");
      for (const diff of report.constantDifferences || []) {
        const row = element(list, "li");
        element(row, "code", `Path ${JSON.stringify(diff.path)}`);
        element(row, "pre", `Left: ${JSON.stringify(diff.left)}\nRight: ${JSON.stringify(diff.right)}`);
      }
    }
    element(root, "p", report.limitation);
    return report;
  }
  async function load(payload, root, options = {}) {
    invalidate(root, "Verifying deployment…");
    const controller = new AbortController();
    requests.set(root, controller);
    const current = () => requests.get(root) === controller && !controller.signal.aborted && (options.isCurrent?.() ?? true);
    try {
      const response = await (options.request || fetch)("/api/v1/verify", {
        method: "POST", headers: { "Content-Type": "application/json" },
        body: JSON.stringify(payload), signal: controller.signal,
      });
      const report = await response.json();
      if (!current()) return;
      if (!response.ok) throw new Error(report.error?.message || "Verification request failed");
      return render(report, root);
    } catch (error) {
      if (current()) invalidate(root, `Verification unavailable: ${error.message}. No match was inferred.`);
    }
  }
  return { render, load, invalidate };
})();
if (typeof module !== "undefined") module.exports = Verify;
