"use strict";
// Adversarial transaction experiments over the Play engine. Every result is
// synthetic (nodeValidated: false). This shows which input scripts still accept
// after a spender-chosen reordering or substitution; it never asserts that a
// live contract is exploitable.
const Attack = (() => {
  function render(response, root) {
    root.replaceChildren();
    root.hidden = false;
    const doc = root.ownerDocument;
    const note = doc.createElement("p");
    note.className = "hint";
    note.textContent =
      "Synthetic experiment — not node-validated. Shows which scripts accepted the mutated draft.";
    root.appendChild(note);
    const table = doc.createElement("table");
    table.className = "probes";
    const body = doc.createElement("tbody");
    for (const d of response.diff || []) {
      const tr = doc.createElement("tr");
      tr.dataset.changed = d.changed ? "yes" : "no";
      const cells = [
        `input ${d.boxId.slice(0, 8)}…`,
        `${d.before} → ${d.after}`,
        d.changed ? "changed" : "unchanged",
      ];
      for (const c of cells) {
        const td = doc.createElement("td");
        td.textContent = c;
        tr.appendChild(td);
      }
      body.appendChild(tr);
    }
    table.appendChild(body);
    root.appendChild(table);
    return response;
  }

  // Post a drafted transaction plus operations to the attack route.
  async function run(draft, operations, request = (...a) => fetch(...a)) {
    const res = await request("/api/v2/attack", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ ...draft, operations }),
    });
    const json = await res.json();
    if (!res.ok) throw new Error(json.error?.message || "Attack request failed");
    return json;
  }

  function bind(document, request = (...a) => fetch(...a)) {
    const root = document.getElementById("attack-result");
    const message = document.getElementById("attack-message");
    const buttons = document.querySelectorAll("[data-attack-op]");
    async function drive(draftJson, ops) {
      root.replaceChildren();
      root.hidden = true;
      message.textContent = "Running the experiment…";
      try {
        const response = await run(JSON.parse(draftJson), ops, request);
        render(response, root);
        message.textContent =
          "Synthetic result. It says which scripts accepted this draft, nothing about a deployed contract.";
      } catch (e) {
        message.textContent = e.message;
      }
    }
    for (const b of buttons) {
      b.onclick = () => {
        const draftEl = document.getElementById("attack-draft");
        const ops = JSON.parse(b.dataset.attackOp);
        return drive(draftEl.value, ops);
      };
    }
    return drive;
  }
  return { render, run, bind };
})();
if (typeof module !== "undefined") module.exports = Attack;
