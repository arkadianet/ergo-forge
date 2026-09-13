"use strict";
const Explain = (() => {
  const requests = new WeakMap();
  function element(root, tag, text) {
    const el = root.ownerDocument.createElement(tag);
    if (text != null) el.textContent = text;
    root.appendChild(el); return el;
  }
  function invalidate(root, message = "Select an expression in Write, then click Explain.") {
    requests.get(root)?.abort(); requests.delete(root);
    root.replaceChildren(); element(root, "p", message);
  }
  // CodeMirror indices are UTF-16; API intervals are UTF-8 bytes.
  function selection(source, from, to) {
    if (!Number.isInteger(from) || !Number.isInteger(to) || from < 0 || to > source.length || from >= to) throw new Error("Select some source text first.");
    const split = i => i > 0 && i < source.length && /[\uD800-\uDBFF]/.test(source[i - 1]) && /[\uDC00-\uDFFF]/.test(source[i]);
    if (split(from) || split(to)) throw new Error("Selection splits a Unicode character.");
    return { offset: new TextEncoder().encode(source.slice(0, from)).length, length: new TextEncoder().encode(source.slice(from, to)).length };
  }
  function payload(scenario, source, params, network, range) {
    if (!scenario || typeof scenario !== "object" || Array.isArray(scenario)) throw new Error("Scenario must be a JSON object.");
    if (scenario.tree != null || (scenario.source != null && scenario.source !== source)) throw new Error("Explain needs the editor source in this scenario; remove its tree or different source first.");
    return { scenario: { ...scenario, source, params: scenario.params ?? params, network: scenario.network || network }, selection: range };
  }
  function render(report, root, options = {}) {
    root.replaceChildren();
    element(root, "p", "Synthetic sandbox reduction; nodeValidated: false. Values and residuals come from this full scenario run.");
    element(root, "p", `${report.message}. Map: ${report.mapStatus}. Selection rule: ${report.selectionRule}.`);
    if (report.error) element(root, "p", `Reduction error: ${report.error}`);
    const expr = report.expression;
    if (!expr) return;
    element(root, "strong", `${expr.opcode}, IR ${expr.irId}`);
    if (expr.span) {
      const text = `line ${expr.span.line}, col ${expr.span.col}, byte ${expr.span.offset} (start position only)`;
      if (options.onSelect) { const b = element(root, "button", text); b.type = "button"; b.className = "secondary tiny"; b.addEventListener("click", () => options.onSelect(expr.span)); }
      else element(root, "p", text);
    } else element(root, "p", "This root has no compiler source position.");
    if (expr.residual != null) element(root, "pre", `Residual: ${expr.residual}`);
    const list = element(root, "ol");
    for (const v of expr.values) {
      const li = element(list, "li"); element(li, "pre", v.value);
      if (v.residual != null) element(li, "pre", `Residual: ${v.residual}`);
      if (v.truncated) element(li, "p", "The engine truncated this recorded value.");
      if (v.residualNote) element(li, "p", v.residualNote);
    }
  }
  async function load(payload, root, options = {}) {
    invalidate(root, "Explaining the selected expression…");
    const controller = new AbortController(); requests.set(root, controller);
    const current = () => requests.get(root) === controller && !controller.signal.aborted && (options.isCurrent?.() ?? true);
    try {
      const response = await (options.request || fetch)("/api/v1/explain", { method: "POST", headers: { "Content-Type": "application/json" }, body: JSON.stringify(payload), signal: controller.signal });
      const report = await response.json();
      if (!current()) return;
      if (!response.ok) throw new Error(report.error?.message || `Request failed (${response.status})`);
      render(report, root, options); return report;
    } catch (error) { if (current()) invalidate(root, `Explanation unavailable: ${error.message}`); }
  }
  return { selection, payload, render, load, invalidate };
})();
if (typeof module !== "undefined") module.exports = Explain;
