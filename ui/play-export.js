"use strict";
// Each control owns the exact request that produced its displayed result.
const PlayExport = (() => {
  const renders = new WeakMap();
  function download(document, value, filename) {
    const blob = new Blob([JSON.stringify(value, null, 2) + "\n"], { type: "application/json" });
    const url = URL.createObjectURL(blob);
    const anchor = document.createElement("a");
    anchor.href = url; anchor.download = filename;
    document.body.appendChild(anchor); anchor.click(); anchor.remove();
    setTimeout(() => URL.revokeObjectURL(url), 1000);
  }
  function render(request, result, root, options = {}) {
    const snapshot = JSON.parse(JSON.stringify(request));
    const generation = {};
    renders.set(root, generation);
    root.replaceChildren();
    const doc = root.ownerDocument;
    const note = doc.createElement("p");
    note.textContent = "Synthetic sandbox verdicts; not node-validated. Save one input’s evaluation. Signing material is omitted; signed inputs use the verdict from a signature-less run.";
    root.appendChild(note);
    const status = doc.createElement("p"); status.setAttribute("role", "status");
    result.inputs.forEach((input, inputIndex) => {
      const row = doc.createElement("div");
      const label = doc.createElement("span");
      label.textContent = `Input ${inputIndex} · ${input.verdict} `;
      row.appendChild(label);
      for (const [kind, title, filename] of [["test", "Save as test", "contract.test.json"], ["scenario", "Save as scenario", "scenario.json"]]) {
        const button = doc.createElement("button");
        button.type = "button"; button.className = "secondary tiny"; button.textContent = title;
        button.onclick = async () => {
          button.disabled = true; status.textContent = "Preparing synthetic export…";
          try {
            const response = await (options.request || fetch)("/api/v1/play/export", {
              method: "POST", headers: { "Content-Type": "application/json" },
              body: JSON.stringify({ ...snapshot, inputIndex, kind }),
            });
            const value = await response.json();
            if (renders.get(root) !== generation) return;
            if (!response.ok) throw new Error(value.error?.message || "Export failed");
            if (value.nodeValidated !== false || value.synthetic !== true) throw new Error("Missing synthetic export labels");
            (options.download || ((value, name) => download(doc, value, name)))(value, filename);
            status.textContent = `Saved ${filename}. Synthetic; run with ergo-es ${kind === "test" ? "test" : "eval"} ${filename}.`;
          } catch (error) {
            if (renders.get(root) === generation) status.textContent = `Export unavailable: ${error.message}`;
          } finally { button.disabled = false; }
        };
        row.appendChild(button);
      }
      root.appendChild(row);
    });
    root.appendChild(status);
  }
  return { render, download };
})();
if (typeof module !== "undefined") module.exports = PlayExport;
