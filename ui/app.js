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

// ── write mode: editor, params, compile ─────────────────────────────────

let lastCompiled = null; // { treeHex } for the scenario panel
const paramTypes = ["Int", "Long", "Coll[Byte]", "SigmaProp", "GroupElement", "Boolean", "Byte", "Short", "BigInt", "Coll[Long]", "String"];

function setMode(mode) {
  const write = mode === "write";
  $("write").hidden = !write;
  $("read").hidden = write;
  $("mode-write").classList.toggle("active", write);
  $("mode-read").classList.toggle("active", !write);
  $("mode-write").setAttribute("aria-selected", String(write));
  $("mode-read").setAttribute("aria-selected", String(!write));
}
$("mode-write").addEventListener("click", () => setMode("write"));
$("mode-read").addEventListener("click", () => setMode("read"));

/// Current parameter values from the form, in the API's typed-value shape.
function collectParams() {
  const out = {};
  for (const tr of $("params-rows").children) {
    const name = tr.dataset.name;
    const type = tr.querySelector("select").value;
    const raw = tr.querySelector("input").value.trim();
    if (!raw) continue;
    let value = raw;
    if (["Int", "Long", "Short", "Byte"].includes(type)) value = Number(raw);
    else if (type === "Boolean") value = raw === "true";
    else if (type === "Coll[Long]") value = raw.split(",").map((x) => Number(x.trim()));
    out[name] = { type, value };
  }
  return out;
}

/// Build (or extend) the params form from [{name, typeHint}], keeping any
/// values already typed in.
function renderParams(needs) {
  const rows = $("params-rows");
  const existing = new Map([...rows.children].map((tr) => [tr.dataset.name, tr]));
  for (const n of needs) {
    if (existing.has(n.name)) continue;
    const tr = document.createElement("tr");
    tr.dataset.name = n.name;
    const td1 = document.createElement("td");
    td1.innerHTML = "<code>$" + n.name + "</code>";
    const td2 = document.createElement("td");
    const sel = document.createElement("select");
    for (const t of paramTypes) {
      const o = document.createElement("option");
      o.value = t; o.textContent = t;
      sel.appendChild(o);
    }
    sel.value = paramTypes.includes(n.typeHint) ? n.typeHint : guessType(n.name);
    td2.appendChild(sel);
    const td3 = document.createElement("td");
    const inp = document.createElement("input");
    inp.type = "text"; inp.placeholder = placeholderFor(sel.value);
    inp.setAttribute("aria-label", "value of " + n.name);
    if (n.default != null) { inp.value = n.default; inp.title = "declared default"; }
    sel.addEventListener("change", () => { inp.placeholder = placeholderFor(sel.value); });
    td3.appendChild(inp);
    tr.append(td1, td2, td3);
    rows.appendChild(tr);
  }
  $("params-panel").hidden = rows.children.length === 0;
}

function guessType(name) {
  const n = name.toLowerCase();
  if (/nft|id|hash|bytes|script|tree|token/.test(n)) return "Coll[Byte]";
  if (/address|base58|base64/.test(n)) return "String";
  if (/^[A-Z][A-Z0-9_]+$/.test(name)) return "String";
  return "Long";
}
function placeholderFor(type) {
  return { "Coll[Byte]": "hex bytes", SigmaProp: "33-byte pubkey hex", GroupElement: "33-byte point hex",
           String: "text substituted inside the string", Boolean: "true / false", BigInt: "decimal",
           "Coll[Long]": "1, 2, 3" }[type] || "number";
}

/// Show a caret under the editor at a byte offset.
function showCaret(offset) {
  const src = $("editor").value;
  const caret = $("caret");
  if (offset == null || offset > src.length) { caret.hidden = true; return; }
  const before = src.slice(0, offset);
  const line = before.split("\n").length;
  const col = offset - before.lastIndexOf("\n") - 1;
  const text = src.split("\n")[line - 1] || "";
  caret.textContent = `line ${line}, col ${col + 1}\n${text}\n${" ".repeat(col)}^`;
  caret.hidden = false;
}

let compileInFlight = false;
async function compile() {
  if (compileInFlight) return;
  const status = $("compile-status");
  const source = $("editor").value;
  compileInFlight = true;
  $("compile").disabled = true;
  status.textContent = "Compiling…";
  status.hidden = false;
  $("caret").hidden = true;
  try {
    const res = await fetch("/api/v1/compile", {
      method: "POST",
      headers: { "content-type": "application/json" },
      body: JSON.stringify({ source, network: $("write-network").value, params: collectParams() }),
    });
    const body = await res.json();
    if (!res.ok) {
      const e = body.error || {};
      if (e.code === "missing_params") {
        renderParams(e.missingParams || []);
        status.textContent = "Fill in the parameters below, then compile again.";
        $("params-rows").querySelector("input")?.focus();
      } else {
        status.textContent = `Error: ${e.message || res.status}`;
        if (e.offset != null) showCaret(e.offset);
      }
      $("compiled").hidden = true;
      return;
    }
    renderParams(body.params.map((p) => ({ name: p.name, typeHint: p.typeHint, default: p.default })));
    renderCompiled(body);
    status.hidden = true;
    lastCompiled = body;
    huntTree(body.treeHex, $("write-network").value);
  } catch (e) {
    status.textContent = `Request failed: ${e}`;
  } finally {
    compileInFlight = false;
    $("compile").disabled = false;
  }
}

/// Select the finding's text in the editor. The compiler cites a start
/// offset (a caret), which for a property read sits on the field name; the
/// snippet is the whole expression. Prefer the snippet occurrence that
/// contains the offset, else the identifier at the offset.
function selectInEditor(offset, snippet) {
  const ed = $("editor");
  const src = ed.value;
  const text = snippet.replace(/…$/, "");
  let start = -1, end = -1;
  if (text) {
    let i = src.indexOf(text);
    while (i >= 0) {
      if (i <= offset && offset < i + text.length) { start = i; end = i + text.length; break; }
      i = src.indexOf(text, i + 1);
    }
  }
  if (start < 0) {
    const m = src.slice(offset).match(/^[A-Za-z0-9_$]+/);
    start = offset; end = offset + (m ? m[0].length : 1);
  }
  ed.focus();
  ed.setSelectionRange(start, Math.min(end, src.length));
  const line = src.slice(0, start).split("\n").length;
  const lineHeight = parseFloat(getComputedStyle(ed).lineHeight) || 18;
  ed.scrollTop = Math.max(0, (line - 3) * lineHeight);
}

function renderCompiled(c) {
  $("c-tree").textContent = c.treeHex;
  $("c-p2s").textContent = c.p2s;
  $("c-p2sh").textContent = c.p2sh;
  $("c-source").textContent = c.source;
  const list = $("c-findings");
  list.textContent = "";
  for (const f of c.findings) {
    const li = document.createElement("li");
    li.dataset.severity = f.severity;
    const chip = document.createElement("span");
    chip.className = `chip ${f.severity}`; chip.textContent = f.severity.toUpperCase();
    const lint = document.createElement("span"); lint.className = "lint"; lint.textContent = f.lint;
    const msg = document.createElement("div"); msg.textContent = f.message;
    const snip = document.createElement("code"); snip.className = "snippet"; snip.textContent = f.snippet;
    li.append(chip, lint, msg, snip);
    if (f.offset != null) {
      const where = document.createElement("span");
      where.className = "where";
      where.textContent = `line ${f.line}, col ${f.col}`;
      li.appendChild(where);
      li.tabIndex = 0;
      li.title = "Click to select in the editor";
      const select = () => selectInEditor(f.offset, f.snippet);
      li.addEventListener("click", select);
      li.addEventListener("keydown", (e) => { if (e.key === "Enter") select(); });
    }
    list.appendChild(li);
  }
  $("c-no-findings").hidden = c.findings.length > 0;
  $("c-positioned").hidden = c.positioned || c.findings.length === 0;
  $("c-hunt").textContent = "Hunting…";
  $("c-hunt").className = "hunt-verdict";
  $("compiled").hidden = false;
}

async function huntTree(treeHex, network) {
  try {
    const res = await fetch("/api/v1/hunt", {
      method: "POST", headers: { "content-type": "application/json" },
      body: JSON.stringify({ input: treeHex, network }),
    });
    const h = await res.json();
    const [label, cls] = HUNT_VERDICTS[h.verdict] || [h.verdict, "neutral"];
    const el = $("c-hunt");
    el.textContent = label + (h.selfSynthetic && h.verdict === "notUnderProbes" ? " — SELF was synthetic; use Read mode with a real box for more" : "");
    el.className = `hunt-verdict ${cls}`;
    if (h.residuals && h.residuals.length) el.textContent += ` · requires: ${h.residuals.join(" | ")}`;
  } catch (e) {
    $("c-hunt").textContent = `Hunt failed: ${e}`;
  }
}

async function loadExamples() {
  try {
    const res = await fetch("/api/v1/examples");
    const items = await res.json();
    const pick = $("example-pick");
    let group = null, og = null;
    for (const it of items) {
      if (it.group !== group) {
        group = it.group;
        og = document.createElement("optgroup");
        og.label = group || "misc";
        pick.appendChild(og);
      }
      const o = document.createElement("option");
      o.value = it.id; o.textContent = it.name;
      og.appendChild(o);
    }
  } catch (e) { /* the gallery is optional */ }
}

$("example-pick").addEventListener("change", async (e) => {
  const id = e.target.value;
  if (!id) return;
  const res = await fetch(`/api/v1/examples/${id}`);
  if (!res.ok) return;
  const ex = await res.json();
  $("editor").value = ex.source;
  $("params-rows").textContent = "";
  $("compiled").hidden = true;
  $("caret").hidden = true;
  renderParams(ex.params);
  const status = $("compile-status");
  if (ex.template) {
    status.textContent = `EIP-5 @contract template — ${ex.params.length} parameter(s), declared defaults prefilled. Compile to instantiate it.`;
    status.hidden = false;
  } else if (ex.params.length) {
    status.textContent = `Needs ${ex.params.length} parameter(s) — fill them in and compile.`;
    status.hidden = false;
  } else {
    status.hidden = true;
  }
  e.target.value = "";
});

$("compile").addEventListener("click", compile);
$("editor").addEventListener("keydown", (e) => {
  if ((e.ctrlKey || e.metaKey) && e.key === "Enter") compile();
});
loadExamples();

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
  if (scenario && typeof scenario === "object" && scenario.source == null && scenario.tree == null) {
    if (!lastCompiled) {
      status.textContent = "Compile something in Write mode first, or give the scenario a source or tree.";
      status.hidden = false;
      return;
    }
    scenario.tree = lastCompiled.treeHex;
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
