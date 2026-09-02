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

/// Rebuild the source pane, wrapping the first occurrence of `snippet` (if
/// any) in a <mark>. Snippets are whitespace-collapsed to one line and may
/// be cut with a trailing ellipsis, so match on a collapsed copy of the
/// source and map the hit back to original offsets.
function showSource(source, snippet) {
  const pre = $("source");
  pre.textContent = "";
  const needle = snippet ? snippet.replace(/…$/, "") : "";
  let at = -1, len = 0;
  if (needle) {
    // collapsed[i] came from source[map[i]].
    let collapsed = "";
    const map = [];
    let inWs = false;
    for (let i = 0; i < source.length; i++) {
      if (/\s/.test(source[i])) {
        if (!inWs) { collapsed += " "; map.push(i); inWs = true; }
      } else { collapsed += source[i]; map.push(i); inWs = false; }
    }
    const ci = collapsed.indexOf(needle);
    if (ci >= 0) {
      at = map[ci];
      len = map[ci + needle.length - 1] + 1 - at;
    }
  }
  if (at < 0) {
    pre.textContent = source;
    return;
  }
  pre.appendChild(document.createTextNode(source.slice(0, at)));
  const m = document.createElement("mark");
  m.textContent = source.slice(at, at + len);
  pre.appendChild(m);
  pre.appendChild(document.createTextNode(source.slice(at + len)));
  m.scrollIntoView({ block: "nearest" });
}

let currentSource = "";

function render(r) {
  currentSource = r.source;
  showSource(r.source, "");
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

    li.tabIndex = 0;
    li.title = "Click to highlight in the source";
    const focus = () => {
      for (const other of list.children) other.classList.remove("active");
      li.classList.add("active");
      showSource(currentSource, f.snippet);
    };
    li.addEventListener("click", focus);
    li.addEventListener("keydown", (e) => { if (e.key === "Enter") focus(); });

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
  const dataRaw = $("data-inputs").value.trim();
  if (dataRaw) {
    try {
      req.dataInputs = JSON.parse(dataRaw);
    } catch (e) {
      verdictEl.textContent = `Data inputs JSON does not parse: ${e.message}`;
      return;
    }
    if (!Array.isArray(req.dataInputs)) {
      verdictEl.textContent = "Data inputs must be a JSON array of boxes.";
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
$("examples").addEventListener("change", (e) => {
  if (!e.target.value) return;
  $("input").value = e.target.value;
  $("network").value = "mainnet";
  e.target.value = "";
  read();
});
$("read").addEventListener("click", read);
$("input").addEventListener("keydown", (e) => {
  if (e.key === "Enter") read();
});
