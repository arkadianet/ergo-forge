"use strict";
const EvidenceReplay = (() => {
  function render(response, root) {
    root.replaceChildren();
    root.hidden = false;
    if (response.apiVersion !== 2 || response.result?.formatVersion !== 1 || !response.result.bundle) {
      throw new Error("Unsupported replay response");
    }
    const r = response.result;
    function line(label, value, cls) {
      const el = root.ownerDocument.createElement("p");
      el.textContent = `${label}: ${value}`;
      if (cls) el.className = cls;
      root.appendChild(el);
    }
    const confirmed = r.nodeValidated === true && r.status === "confirmed-violation" && r.claim?.status === "confirmed-violation";
    line("Result", confirmed ? "Node-validated property violation" : r.status, confirmed ? "node-claim-badge" : "replay-status");
    line("Claim scope", r.scope);
    if (r.claim?.scope) line("Property claim scope", r.claim.scope);
    if (r.execution?.scope) line("Execution scope", r.execution.scope);
    line("Historical state", "Not established by replay; source-recorded inputs do not prove historical unspentness or inclusion.");
    const boxes = r.bundle.execution.case.premises.boxes;
    const origins = boxes.status === "present" ? [...new Set(boxes.value.map(b => b.origin))] : ["missing"];
    line("Input state origins", origins.join(", "));
    line("Hypothetical state", origins.includes("hypothetical") ? "Includes hypothetical inputs" : "See per-premise origins below; no historical-state assertion");
    line("Bundle fingerprint", r.bundleFingerprint);
    line("Property version", r.bundle.property.version);
    if (r.execution?.transactionId) line("Transaction ID", r.execution.transactionId);
    function details(label, value) {
      const section = root.ownerDocument.createElement("details");
      const summary = root.ownerDocument.createElement("summary");
      summary.textContent = label;
      const pre = root.ownerDocument.createElement("pre");
      pre.textContent = JSON.stringify(value, null, 2);
      section.append(summary, pre);
      root.appendChild(section);
    }
    details("Declared property", r.bundle.property);
    if (r.claim?.accounting || r.accounting) details("Property accounting", r.claim?.accounting || r.accounting);
    if (r.reason) line("Reason", r.reason);
    if (r.execution?.detail) line("Validation detail", r.execution.detail);
    const missing = [], provenance = [];
    function walk(value, path) {
      if (!value || typeof value !== "object") return;
      if (value.status === "missing" && typeof value.reason === "string") missing.push(`${path}: ${value.reason}`);
      if (typeof value.origin === "string") provenance.push(`${path}: ${value.origin}`);
      for (const [key, child] of Object.entries(value)) walk(child, `${path}.${key}`);
    }
    walk(r.bundle, "bundle");
    line("Missing premises", missing.length ? "Listed below (including residual premises on accepted results)" : "None recorded; scope restrictions still apply");
    const list = root.ownerDocument.createElement("ul");
    list.className = "replay-missing-premises";
    for (const text of missing) {
      const item = root.ownerDocument.createElement("li");
      item.textContent = text;
      list.appendChild(item);
    }
    root.appendChild(list);
    for (const origin of provenance) line("Premise origin", origin);
  }
  function bind(document, request = (...args) => fetch(...args)) {
    const go = document.getElementById("replay-go");
    const root = document.getElementById("replay-result");
    const message = document.getElementById("replay-message");
    go.onclick = async () => {
      root.replaceChildren();
      root.hidden = true;
      go.disabled = true;
      message.textContent = "Replaying supplied evidence…";
      try {
        const file = document.getElementById("replay-file").files?.[0];
        if (!file) throw new Error("Choose an evidence JSON file first.");
        if (file.size > 1024 * 1024) throw new Error("Evidence file exceeds the 1 MiB request limit.");
        const saved = JSON.parse(await file.text());
        // Saved verdicts are discarded. Only the bundle is submitted for fresh replay.
        const bundle = saved.apiVersion === 2 ? saved.result?.bundle : (saved.bundle || saved);
        const res = await request("/api/v2/replay", {
          method: "POST", headers: { "Content-Type": "application/json" }, body: JSON.stringify(bundle),
        });
        const response = await res.json();
        if (!res.ok) throw new Error(response.error?.message || "Evidence replay request failed");
        render(response, root);
        message.textContent = "Replay complete. Results apply only to the supplied premises and property.";
      } catch (error) {
        root.replaceChildren();
        root.hidden = true;
        message.textContent = error.message;
      } finally {
        go.disabled = false;
      }
    };
    return go.onclick;
  }
  return { render, bind };
})();
if (typeof module !== "undefined") module.exports = EvidenceReplay;
