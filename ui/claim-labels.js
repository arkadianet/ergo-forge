// Shared DOM rendering for the existing scenario/preflight views. No claim promotion.
"use strict";
const ClaimLabels = (() => {
  const verdicts = {
    spendableByAnyone: ["Sample passed without a proof", "warn"],
    movableByAnyone: ["Preserving-output sample passed", "warn"],
    requiresProof: ["Proof requirements observed under these probes", "neutral"],
    notUnderProbes: ["No sample passed (not a proof of safety)", "neutral"],
  };
  const synthetic = "SELF was synthetic (no registers, value 0). Supply a box to test its scenario; full node validation has not run.";
  function renderHunt(h, verdict, warning) {
    const [label, cls] = verdicts[h.verdict] || [h.verdict, "neutral"];
    verdict.textContent = `${label} — full node validation has not run`;
    verdict.className = `hunt-verdict ${cls}`;
    if (warning) {
      warning.textContent = synthetic;
      warning.hidden = !h.selfSynthetic;
    } else if (h.selfSynthetic) verdict.textContent += ` — ${synthetic}`;
  }
  function renderPreflight(body, verdict) {
    const passed = body.preflightPassed ?? body.valid; // deprecated v1 alias is still preflight
    verdict.textContent = `${passed ? "Preflight passed" : "Preflight failed"} — full node validation has not run. ${body.signaturesNeeded} signature(s) needed. ERG in ${body.ergIn}, out ${body.ergOut}, at height ${body.height}.`;
    verdict.className = `hunt-verdict ${passed ? "neutral" : "bad"}`;
  }
  function renderTriage(triage, note) {
    if (!triage) {
      note.textContent = "unconfirmed: Static observation only; full node validation has not run.";
    } else if (triage.formatVersion !== 2 || triage.state === "confirmed" || triage.nodeValidated !== false) {
      note.textContent = "Legacy/unverified record: replay its request. Stored labels do not establish node validation.";
    } else {
      note.textContent = `${triage.state}: ${triage.explanation} Full node validation has not run.`;
    }
  }
  return { renderHunt, renderPreflight, renderTriage };
})();
if (typeof module !== "undefined") module.exports = ClaimLabels;
