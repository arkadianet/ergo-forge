"use strict";
// Review observations, never a score. All server/caller text is inserted as text.
const Checklist = (() => {
  const requests = new WeakMap();
  function element(root, tag, text, className) {
    const el = root.ownerDocument.createElement(tag);
    if (text != null) el.textContent = text;
    if (className) el.className = className;
    root.appendChild(el);
    return el;
  }

  function observation(root, finding, select) {
    const line = element(root, "li", null, "checklist-observation");
    element(line, "span", "static", "chip");
    element(line, "p", finding.text);
    const anchor = finding.anchor || {};
    const location = `${finding.lint} · node ${anchor.nodeId ?? "unavailable"} · IR ${anchor.irId ?? "unavailable"}`;
    element(line, "small", location);
    element(line, "code", finding.snippet, "snippet");
    if (select) {
      const button = element(line, "button", "Show in recovered source", "secondary tiny");
      button.type = "button";
      button.onclick = () => select(finding);
    }
    return line;
  }

  function render(response, root, select) {
    root.replaceChildren();
    root.hidden = false;
    element(root, "p", "Static observations set review priority; they do not establish vulnerabilities. Unchecked questions remain open. Artifact associations are caller-supplied.", "hint");
    if (response.completeness && response.completeness !== "complete") {
      element(root, "p", "Recovery is partial. Static observations cover only the recovered code; missing findings prove nothing.", "hint");
    }
    const list = element(root, "ul", null, "checklist-rows");
    for (const row of response.rows || []) {
      const li = element(list, "li");
      li.dataset.vectorId = row.id;
      li.dataset.provenance = row.provenance;
      element(li, "span", row.provenance, "chip");
      element(li, "strong", row.title);
      element(li, "p", row.answer);
      const findings = element(li, "ul");
      for (const finding of row.findings || []) observation(findings, finding, select);
      for (const fingerprint of row.artifactFingerprints || []) {
        const artifact = (response.artifacts || []).find(a => a.fingerprint === fingerprint);
        if (artifact) {
          const detail = element(li, "details");
          element(detail, "summary", `${artifact.provenance} · supplied artifact`);
          element(detail, "p", artifact.answer);
          element(detail, "code", artifact.fingerprint, "snippet");
          element(detail, "pre", JSON.stringify(artifact.result, null, 2));
        }
      }
    }
    for (const artifact of response.artifacts || []) {
      if (!artifact.vectorIds?.length) {
        element(root, "p", `${artifact.provenance}: ${artifact.answer} No vector association supplied.`, "hint");
      }
    }
    return response;
  }

  function invalidate(root, message = "Compile or read the current contract to load its checklist.") {
    requests.get(root)?.abort();
    requests.delete(root);
    root.replaceChildren();
    element(root, "p", message, "hint");
  }

  function renderNegativeSpace(lines, root, select) {
    root.replaceChildren();
    root.hidden = false;
    element(root, "p", "Static observations from the existing review instruments. These describe recognised omissions, not vulnerabilities or a complete account of what the contract checks.", "hint");
    const list = element(root, "ul", null, "negative-space-lines");
    for (const line of lines || []) observation(list, line, select);
    if (!lines?.length) element(root, "p", "No observations from these instruments. This does not establish that every box, output or register is constrained.", "hint");
  }

  async function load(input, network, root, options = {}) {
    invalidate(root, "Loading checklist…");
    const controller = new AbortController();
    requests.set(root, controller);
    const current = () => requests.get(root) === controller && !controller.signal.aborted && (options.isCurrent?.() ?? true);
    const request = options.request || ((...args) => fetch(...args));
    try {
      const response = await request("/api/v1/checklist", {
        method: "POST", headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ input, network }), signal: controller.signal,
      });
      const body = await response.json();
      if (!current()) return;
      if (!response.ok) throw new Error(body.error?.message || "Checklist request failed");
      render(body, root, options.select);
      return body;
    } catch (error) {
      if (current()) invalidate(root, `Checklist unavailable: ${error.message}. No answers were inferred.`);
    }
  }

  return { render, load, invalidate, renderNegativeSpace };
})();
if (typeof module !== "undefined") module.exports = Checklist;
