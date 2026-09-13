"use strict";
// The server decides ownership. Candidate costs are never split among spans.
const CostSpans = (() => {
  function element(root, tag, text) {
    const el = root.ownerDocument.createElement(tag);
    if (text != null) el.textContent = text;
    root.appendChild(el); return el;
  }
  function invalidate(root, message = "Run the current scenario to see cost by source.") {
    root.replaceChildren(); element(root, "p", message);
  }
  function render(report, root, options = {}) {
    invalidate(root, "");
    if (!report) { invalidate(root, "Cost tracing is unavailable on this server."); return; }
    const total = report.attributedJit + report.ambiguousJit + report.unattributedJit;
    if (total !== report.totalJit || report.rows.reduce((sum, r) => sum + r.jit, 0) !== total) {
      invalidate(root, "Cost totals did not reconcile; no source cost was displayed."); return;
    }
    element(root, "p", `${total.toLocaleString()} JIT units; ${(report.exactShare * 100).toFixed(1)}% exactly attributed. Exact ${report.attributedJit}; ambiguous ${report.ambiguousJit}; unattributed ${report.unattributedJit}.`);
    element(root, "p", "Synthetic sandbox reduction; nodeValidated: false. Shares use this diagnostic trace, excluding proof verification. Compiler spans are start positions only; links select the token there.");
    if (options.partial) element(root, "p", "Reduction stopped with an error; the trace may be partial.");
    if (report.mapStatus !== "aligned") element(root, "p", `Source map: ${report.mapStatus}. No source costs were attributed.`);
    if (!total) element(root, "p", "No cost was recorded; the exact share is 0%.");
    const list = element(root, "ul");
    for (const row of report.rows) {
      const li = element(list, "li"); li.dataset.rule = row.rule;
      element(li, "strong", `${row.rule}: ${row.label} — ${row.jit.toLocaleString()} JIT (${(row.share * 100).toFixed(1)}%)`);
      element(li, "p", row.reason);
      for (const candidate of row.candidates || []) {
        const span = candidate.span;
        const text = `${row.rule === "ambiguous" ? "Candidate " : ""}IR ${candidate.irId}: ${span ? `line ${span.line}, col ${span.col}, byte ${span.offset} (start only)` : "no compiler position"}`;
        if (span && options.onSelect) {
          const button = element(li, "button", text); button.type = "button"; button.className = "secondary tiny";
          button.addEventListener("click", () => options.onSelect(span));
        } else element(li, "p", text);
      }
    }
  }
  return { render, invalidate };
})();
if (typeof module !== "undefined") module.exports = CostSpans;
