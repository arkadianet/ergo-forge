// ergo-web reader — plain JS, no framework.
"use strict";

const $ = (id) => document.getElementById(id);

// One inspection at a time: Enter in the input box and the button share this.
let inFlight = false;
// Hunt requests can overlap (re-run while one is pending); only the latest
// response may render.
let huntGeneration = 0;

async function read() {
  if (inFlight) return;
  const input = $("input").value.trim();
  const network = $("network").value;
  const status = $("status");
  const banner = $("partial-banner");
  const result = $("result");

  if (!input) {
    status.textContent = "Paste an address or ErgoTree hex first.";
    status.hidden = false;
    return;
  }

  inFlight = true;
  $("read").disabled = true;
  status.textContent = "Reading contract…";
  status.hidden = false;
  banner.hidden = true;
  result.hidden = true;

  try {
    const res = await fetch("/api/v1/inspect", {
      method: "POST",
      headers: { "content-type": "application/json" },
      body: JSON.stringify({ input, network }),
    });
    const body = await res.json();
    if (!res.ok) {
      status.textContent = `Error: ${(body.error && body.error.message) || res.status}`;
      return;
    }
    render(body);
    status.hidden = true;
    await huntFor(input, network);
  } catch (e) {
    status.textContent = `Request failed: ${e}`;
  } finally {
    inFlight = false;
    $("read").disabled = false;
  }
}

function render(r) {
  $("source").textContent = r.source;
  $("tree-hex").textContent = r.treeHex;
  $("address").textContent = r.address;

  const banner = $("partial-banner");
  if (r.completeness === "partial") {
    const bits = [];
    if (r.rawPlaceholders > 0) {
      bits.push(`${r.rawPlaceholders} unreadable section(s)`);
    }
    if (r.truncated) {
      bits.push("depth limit hit");
    }
    $("partial-detail").textContent = bits.length ? `${bits.join(", ")} —` : "";
    banner.hidden = false;
  } else {
    banner.hidden = true;
  }

  const list = $("findings");
  list.textContent = "";
  for (const f of r.findings) {
    const li = document.createElement("li");
    li.dataset.severity = f.severity;

    const chip = document.createElement("span");
    chip.className = `chip ${f.severity}`;
    chip.textContent = f.severity.toUpperCase();
    li.appendChild(chip);

    const lint = document.createElement("span");
    lint.className = "lint";
    lint.textContent = f.lint;
    li.appendChild(lint);

    const msg = document.createElement("div");
    msg.textContent = f.message;
    li.appendChild(msg);

    const snip = document.createElement("code");
    snip.className = "snippet";
    snip.textContent = f.snippet;
    li.appendChild(snip);

    list.appendChild(li);
  }
  $("no-findings").hidden = r.findings.length > 0;
  $("result").hidden = false;
}

const HUNT_VERDICTS = {
  spendableByAnyone: ["Spendable by anyone", "bad"],
  movableByAnyone: ["Movable by anyone (funds stay in the contract)", "warn"],
  requiresProof: ["Requires a proof — see who below", "ok"],
  notUnderProbes: ["Not spendable under these probes (not a proof of safety)", "neutral"],
};

async function huntFor(input, network) {
  const verdictEl = $("hunt-verdict");
  verdictEl.textContent = "Hunting…";
  verdictEl.className = "hunt-verdict";
  const req = { input, network };
  const heightRaw = $("height").value.trim();
  if (heightRaw) {
    const height = Number(heightRaw);
    if (!Number.isInteger(height) || height < 1 || height > 0xffffffff) {
      verdictEl.textContent = "Height must be an integer from 1 through 4294967295.";
      return;
    }
    req.height = height;
  }
  const boxRaw = $("self-box").value.trim();
  if (boxRaw) {
    try {
      req.selfBox = JSON.parse(boxRaw);
    } catch (e) {
      verdictEl.textContent = `Box JSON does not parse: ${e.message}`;
      return;
    }
  }
  const generation = ++huntGeneration;
  try {
    const res = await fetch("/api/v1/hunt", {
      method: "POST",
      headers: { "content-type": "application/json" },
      body: JSON.stringify(req),
    });
    const body = await res.json();
    if (generation !== huntGeneration) return; // a newer hunt owns the panel
    if (!res.ok) {
      verdictEl.textContent = `Hunt error: ${(body.error && body.error.message) || res.status}`;
      return;
    }
    renderHunt(body);
  } catch (e) {
    if (generation !== huntGeneration) return;
    verdictEl.textContent = `Hunt request failed: ${e}`;
  }
}

function renderHunt(h) {
  const [label, cls] = HUNT_VERDICTS[h.verdict] || [h.verdict, "neutral"];
  const verdictEl = $("hunt-verdict");
  verdictEl.textContent = label;
  verdictEl.className = `hunt-verdict ${cls}`;
  const undeterminedOnSynthetic = h.selfSynthetic && h.verdict === "notUnderProbes";
  $("hunt-synthetic").hidden = !undeterminedOnSynthetic;
  // The real box is the fix; put the form in front of the user.
  if (undeterminedOnSynthetic) $("self-box").closest("details").open = true;

  const res = $("hunt-residuals");
  res.textContent = "";
  for (const r of h.residuals) {
    const li = document.createElement("li");
    li.textContent = r;
    res.appendChild(li);
  }

  const tb = $("hunt-probes");
  tb.textContent = "";
  for (const p of h.probes) {
    const tr = document.createElement("tr");
    tr.dataset.verdict = p.verdict;
    for (const cell of [
      String(p.height),
      p.output,
      p.verdict,
      p.error ? `error: ${p.error}` : p.reducedTo || "",
    ]) {
      const td = document.createElement("td");
      td.textContent = cell;
      tr.appendChild(td);
    }
    tb.appendChild(tr);
  }
}

// ── scenario eval ────────────────────────────────────────────────────────

const EVAL_VERDICTS = {
  pass: ["PASS — spendable in this context", "bad"],
  fail: ["FAIL — not spendable in this context", "ok"],
  error: ["ERROR — the script threw", "warn"],
  needsProof: ["NEEDS PROOF — a signature is required", "ok"],
  proofAccepted: ["PROOF ACCEPTED", "bad"],
  proofRejected: ["PROOF REJECTED", "ok"],
};

let evalInFlight = false;

async function runScenario() {
  if (evalInFlight) return;
  const status = $("eval-status");
  const result = $("eval-result");
  let scenario;
  try {
    scenario = JSON.parse($("scenario").value);
  } catch (e) {
    status.textContent = `Scenario JSON does not parse: ${e.message}`;
    status.hidden = false;
    return;
  }
  evalInFlight = true;
  $("run").disabled = true;
  status.textContent = "Evaluating…";
  status.hidden = false;
  result.hidden = true;
  try {
    const res = await fetch("/api/v1/eval", {
      method: "POST",
      headers: { "content-type": "application/json" },
      body: JSON.stringify(scenario),
    });
    const body = await res.json();
    if (!res.ok) {
      status.textContent = `Error: ${(body.error && body.error.message) || res.status}`;
      return;
    }
    const [label, cls] = EVAL_VERDICTS[body.verdict] || [body.verdict, "neutral"];
    const v = $("eval-verdict");
    v.textContent = label;
    v.className = `hunt-verdict ${cls}`;
    $("eval-cost").textContent = `${body.cost} / ${body.costLimit} block units`;
    $("eval-reduced").textContent = body.reducedTo || "—";
    $("eval-error").textContent = body.error || "—";
    $("eval-address").textContent = body.address;
    const list = $("eval-trace");
    list.textContent = "";
    for (const t of body.trace) {
      const li = document.createElement("li");
      li.textContent = `${t.label} = ${t.value}`;
      list.appendChild(li);
    }
    status.hidden = true;
    result.hidden = false;
  } catch (e) {
    status.textContent = `Request failed: ${e}`;
  } finally {
    evalInFlight = false;
    $("run").disabled = false;
  }
}

$("run").addEventListener("click", runScenario);

$("rehunt").addEventListener("click", () => {
  const input = $("input").value.trim();
  if (input) huntFor(input, $("network").value);
});
$("read").addEventListener("click", read);
$("input").addEventListener("keydown", (e) => {
  if (e.key === "Enter") read();
});
