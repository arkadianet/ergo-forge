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

  $("hunt-rent").textContent = h.rent ? rentSentence(h.rent, { network: $("network").value }) : "";
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

// The editor: CodeMirror over the #editor textarea (vendored, no CDN).
const editor = CodeMirror.fromTextArea($("editor"), {
  mode: "ergoscript",
  lineNumbers: true,
  matchBrackets: true,
  styleActiveLine: true,
  indentUnit: 2,
  tabSize: 2,
  lineWrapping: true,
  viewportMargin: 50,
  extraKeys: { "Ctrl-Enter": () => compile(), "Cmd-Enter": () => compile() },
});
const editorValue = () => editor.getValue();
const setEditorValue = (v) => { editor.setValue(v); clearMarks(); };
let marks = [];
function clearMarks() { for (const m of marks) m.clear(); marks = []; }

let lastCompiled = null; // { treeHex } for the scenario panel
const paramTypes = ["Int", "Long", "Coll[Byte]", "SigmaProp", "GroupElement", "Boolean", "Byte", "Short", "BigInt", "Coll[Long]", "String"];

function setMode(mode) {
  for (const m of ["build", "write", "read"]) {
    const on = m === mode;
    $(m).hidden = !on;
    $(`mode-${m}`).classList.toggle("active", on);
    $(`mode-${m}`).setAttribute("aria-selected", String(on));
  }
  $("dev-panels").hidden = mode === "build";
  if (mode === "write") editor.refresh();
}
$("mode-build").addEventListener("click", () => setMode("build"));
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

/// Mark a compile error at a byte offset: a squiggle on the token there and
/// the caret line under the editor (kept for copy/paste of the position).
function showCaret(byteOffset) {
  const src = editorValue();
  const offset = byteOffsetToIndex(src, byteOffset);
  const tok = (src.slice(offset).match(/^[A-Za-z0-9_$.]+/) || [""])[0];
  // At end of input (unexpected EOF) mark the last character instead.
  const start = offset >= src.length ? Math.max(0, src.length - 1) : offset;
  const from = editor.posFromIndex(start);
  const to = editor.posFromIndex(Math.min(src.length, start + Math.max(1, tok.length)));
  marks.push(editor.markText(from, to, { className: "cm-error-mark", title: "compile error here" }));
  editor.scrollIntoView(from, 60);
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
  const source = editorValue();
  compileInFlight = true;
  clearMarks();
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
/// UTF-8 byte offset (what the compiler cites) → UTF-16 code-unit index
/// (what the textarea uses). Equal for ASCII; diverges after any non-ASCII.
function byteOffsetToIndex(src, byteOffset) {
  let bytes = 0;
  for (let i = 0; i < src.length; i++) {
    if (bytes >= byteOffset) return i;
    const cp = src.codePointAt(i);
    bytes += cp < 0x80 ? 1 : cp < 0x800 ? 2 : cp < 0x10000 ? 3 : 4;
    if (cp >= 0x10000) i++; // surrogate pair: two code units
  }
  return src.length;
}

function selectInEditor(byteOffset, snippet) {
  const src = editorValue();
  const offset = byteOffsetToIndex(src, byteOffset);
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
  const from = editor.posFromIndex(start), to = editor.posFromIndex(Math.min(end, src.length));
  editor.focus();
  editor.setSelection(from, to);
  editor.scrollIntoView({ from, to }, 60);
}

/// Squiggle every positioned finding in the editor (severity-coloured).
function markFindings(findings) {
  const src = editorValue();
  for (const f of findings) {
    if (f.offset == null) continue;
    const offset = byteOffsetToIndex(src, f.offset);
    const text = f.snippet.replace(/…$/, "");
    let start = offset, end = offset + 1;
    let i = src.indexOf(text);
    while (i >= 0) { if (i <= offset && offset < i + text.length) { start = i; end = i + text.length; break; } i = src.indexOf(text, i + 1); }
    marks.push(editor.markText(editor.posFromIndex(start), editor.posFromIndex(end),
      { className: `cm-finding-${f.severity}`, title: `${f.lint}: ${f.message}` }));
  }
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
  $("c-rent").textContent = c.rent ? rentSentence(c.rent, { withHeight: false }) : "";
  markFindings(c.findings);
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
  setEditorValue(ex.source);
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
loadExamples();

// ── build mode: recipes → questions → address ────────────────────────────

let chainHeight = null;   // from /api/v1/config when an explorer is configured
let chainHeightAt = 0;    // when that height was observed (ms)
let chainNetwork = null;  // which network the height belongs to
let recipe = null;        // { id, name, doc, params, source }
let built = null;         // last compile result for the wizard

const BLOCK_SECONDS = 120;

function buildStep(n) {
  buildStepEl(`build-step-${n}`);
}

async function loadRecipes() {
  try {
    const items = await (await fetch("/api/v1/examples")).json();
    const box = $("recipes");
    // Simplest first; anything not listed goes after, alphabetically.
    const order = ["time-lock", "inheritance", "two-of-three", "escrow", "refundable-payment", "savings-cap", "subscription", "vesting", "cliff-vesting", "token-sale", "bounty", "price-gate", "burn"];
    const rank = (id) => { const i = order.indexOf(id.split("/").pop()); return i < 0 ? order.length : i; };
    const recipes = items.filter((i) => i.group === "recipes").sort((a, b) => rank(a.id) - rank(b.id) || a.id.localeCompare(b.id));
    for (const it of recipes) {
      const ex = await (await fetch(`/api/v1/examples/${it.id}`)).json();
      const card = document.createElement("button");
      card.type = "button";
      card.className = "recipe";
      const title = document.createElement("strong");
      title.textContent = RECIPE_TITLES[it.name] || humanize(ex.doc ? ex.doc.name : it.name);
      const desc = document.createElement("span");
      desc.textContent = (ex.doc && ex.doc.description.split(/\.\s/)[0] + ".") || "";
      card.append(title, desc);
      card.addEventListener("click", () => startRecipe(ex));
      box.appendChild(card);
    }
    const custom = document.createElement("button");
    custom.type = "button"; custom.className = "recipe custom";
    const t = document.createElement("strong"); t.textContent = "Combine rules yourself";
    const d = document.createElement("span"); d.textContent = "Who may spend, under what conditions — as many ways to spend as you need.";
    custom.append(t, d);
    custom.addEventListener("click", startComposer);
    box.appendChild(custom);
  } catch (e) { /* no gallery, no build mode */ }
}

function buildStepEl(id) {
  for (const k of ["build-step-1", "build-compose", "build-step-2", "build-step-3"]) $(k).hidden = k !== id;
  window.scrollTo({ top: $("build").offsetTop - 12, behavior: "smooth" });
}

const RECIPE_TITLES = {
  "time-lock": "Lock savings until a date",
  "inheritance": "Inheritance / backup access",
  "two-of-three": "Shared account (2 of 3 must agree)",
  "escrow": "Escrow with an arbiter",
  "refundable-payment": "Payment you can take back",
  "savings-cap": "Savings with a spending limit",
  "subscription": "Pay someone regularly from a pot",
  "vesting": "Release funds gradually",
  "cliff-vesting": "Release funds gradually, after a cliff",
  "token-sale": "Sell tokens at a fixed price",
  "bounty": "Bounty for a secret",
  "price-gate": "Spend only above an oracle price",
  "burn": "Burn address",
};

function humanize(name) {
  return name.replace(/([a-z])([A-Z])/g, "$1 $2").replace(/^./, (c) => c.toUpperCase());
}

/// Date→height conversion is only honest for the explorer's own network.
function datesAvailable() {
  return chainHeight != null && chainNetwork === $("build-network").value;
}
/// The chain height now, extrapolated from the observation at ~2 min/block.
function heightNow() {
  return chainHeight + Math.floor((Date.now() - chainHeightAt) / 1000 / BLOCK_SECONDS);
}

/// Which input to show for a template parameter, from its type and name.
function fieldKind(p) {
  const t = p.typeHint || "";
  const n = p.name.toLowerCase();
  if (t === "SigmaProp") return "address";
  if (t === "Coll[Byte]" && /hash/.test(n)) return "secret";
  if ((t === "Int" || t === "Long") && /height|deadline|expiry|unlock|until|after/.test(n)) return "height";
  if (t === "Coll[Byte]" && /nft|token|id/.test(n)) return "tokenId";
  if (t === "Long" && /erg|value|amount|fee/.test(n)) return "erg";
  if (t === "Boolean") return "bool";
  return "text";
}

/// "Question? — help text" from a recipe's @param line.
function splitDoc(d) {
  const [q, ...rest] = (d || "").split(" — ");
  return { question: q || "", help: rest.join(" — ") };
}

/// Client-side sanity check per field kind, so a typo is caught while typing.
function fieldProblem(kind, raw, network) {
  const v = raw.trim();
  if (!v) return "";
  if (kind === "address") {
    if (!/^[1-9A-HJ-NP-Za-km-z]+$/.test(v)) return "An address only has letters and digits (no 0, O, I or l).";
    if (network === "mainnet" && !v.startsWith("9")) return "A mainnet wallet address starts with 9.";
    if (network === "testnet" && !v.startsWith("3")) return "A testnet wallet address starts with 3.";
    if (v.length < 40 || v.length > 60) return "That doesn't look like a wallet address (about 51 characters).";
  }
  if (kind === "tokenId" && !/^[0-9a-fA-F]{64}$/.test(v)) return "A token id is exactly 64 hex characters.";
  if (kind === "erg" && !(Number(v) >= 0)) return "Enter an amount in ERG, like 1.5.";
  return "";
}

function startRecipe(ex) {
  recipe = ex;
  built = null;
  $("wizard-title").textContent = RECIPE_TITLES[ex.name] || (ex.doc ? humanize(ex.doc.name) : ex.name);
  $("wizard-desc").textContent = ex.doc ? ex.doc.description : "";
  const fields = $("wizard-fields");
  fields.textContent = "";
  const network = $("build-network").value;
  for (const p of ex.params) {
    const kind = fieldKind(p);
    const { question, help } = splitDoc(p.description);
    const row = document.createElement("div");
    row.className = "field";
    row.dataset.name = p.name; row.dataset.kind = kind; row.dataset.type = p.typeHint || "Long";
    const label = document.createElement("label");
    label.htmlFor = `f-${p.name}`;
    label.textContent = question || p.name;
    const inp = document.createElement("input");
    inp.id = `f-${p.name}`;
    inp.required = true;
    inp.autocomplete = "off";
    if (kind === "address") { inp.placeholder = network === "testnet" ? "3…" : "9…"; inp.spellcheck = false; }
    else if (kind === "height") {
      inp.type = datesAvailable() ? "datetime-local" : "number";
      inp.placeholder = datesAvailable() ? "" : "block height, e.g. 1900000";
      inp.min = "1";
    }
    else if (kind === "tokenId") { inp.placeholder = "64 hex characters"; inp.spellcheck = false; }
    else if (kind === "secret") { inp.placeholder = "the secret phrase"; inp.autocomplete = "off"; }
    else if (kind === "erg") { inp.type = "number"; inp.step = "0.000000001"; inp.placeholder = "e.g. 1.5"; }
    else if (kind === "bool") { inp.type = "checkbox"; inp.required = false; }
    else { inp.placeholder = p.typeHint === "Long" || p.typeHint === "Int" ? "a whole number" : (p.typeHint || ""); }
    if (p.default != null && kind !== "height") inp.value = p.default;
    const helpEl = document.createElement("span");
    helpEl.className = "hint";
    let helpText = help;
    if (kind === "height") {
      helpText = (help ? help + " " : "") + (datesAvailable()
        ? "Ergo counts time in blocks (about one every 2 minutes); we convert your date to a block."
        : "Enter a block height; there is about one block every 2 minutes. Dates are offered when this instance has an explorer for the chosen network.");
    }
    if (kind === "address" && !help) helpText = "Paste an address from your wallet.";
    if (kind === "secret") helpText = "Type the secret phrase itself. It is hashed here in your browser; only the hash goes into the contract, and the phrase never leaves this page. Whoever knows the phrase can claim — keep it safe.";
    helpEl.textContent = helpText;
    const problem = document.createElement("span");
    problem.className = "problem";
    problem.hidden = true;
    inp.addEventListener("input", () => {
      const msg = fieldProblem(kind, inp.value, $("build-network").value);
      problem.textContent = msg; problem.hidden = !msg;
      inp.classList.toggle("invalid", !!msg);
    });
    row.append(label, inp, helpEl, problem);
    fields.appendChild(row);
  }
  $("build-status").hidden = true;
  buildStep(2);
  const first = fields.querySelector("input");
  if (first) first.focus();
}

/// The wizard's answers as typed parameters, or an error message.
function wizardParams() {
  const out = {};
  for (const row of $("wizard-fields").children) {
    const name = row.dataset.name, kind = row.dataset.kind, type = row.dataset.type;
    const inp = row.querySelector("input");
    const raw = (inp.value || "").trim();
    const label = row.querySelector("label").textContent;
    if (kind === "bool") { out[name] = { type: "Boolean", value: inp.checked }; continue; }
    if (!raw) return { error: `Please answer: ${label}` };
    const problem = fieldProblem(kind, raw, $("build-network").value);
    if (problem) return { error: `${label} — ${problem}` };
    if (kind === "address") out[name] = { type: "SigmaProp", value: raw };
    else if (kind === "height") {
      let h;
      if (inp.type === "datetime-local") {
        const t = new Date(raw).getTime();
        if (Number.isNaN(t)) return { error: `${label} — that date does not parse.` };
        const now = heightNow();
        h = now + Math.ceil((t - Date.now()) / 1000 / BLOCK_SECONDS);
        if (h <= now) return { error: `${label} — the date must be in the future.` };
      } else {
        h = Number(raw);
        if (!Number.isInteger(h) || h < 1) return { error: `${label} — a block height is a whole number.` };
      }
      out[name] = { type, value: h };
    }
    else if (kind === "tokenId") out[name] = { type: "Coll[Byte]", value: raw.toLowerCase() };
    else if (kind === "secret") out[name] = { type: "Coll[Byte]", value: window.blake2b.blake2bHex(new TextEncoder().encode(raw), null, 32) };
    else if (kind === "erg") out[name] = { type: "Long", value: Math.round(Number(raw) * 1e9) };
    else if (type === "Int" || type === "Long" || type === "Short" || type === "Byte") {
      if (!/^-?\d+$/.test(raw)) return { error: `${label} — a whole number, please.` };
      const big = BigInt(raw);
      const safe = big <= BigInt(Number.MAX_SAFE_INTEGER) && big >= -BigInt(Number.MAX_SAFE_INTEGER);
      out[name] = { type, value: safe ? Number(raw) : raw };
    }
    else out[name] = { type, value: raw };
  }
  return { params: out };
}

function shortAddr(a) { a = String(a); return a.length > 16 ? `${a.slice(0, 8)}…${a.slice(-6)}` : a; }
function dateOfHeight(h) {
  const when = new Date(Date.now() + (h - heightNow()) * BLOCK_SECONDS * 1000);
  return when.toLocaleString(undefined, { dateStyle: "medium", timeStyle: "short" });
}

/// Plain-language summary: each answer restated under its question.
function describeBuild(params) {
  const lines = [];
  for (const row of $("wizard-fields").children) {
    const name = row.dataset.name, kind = row.dataset.kind;
    const label = row.querySelector("label").textContent.replace(/\?$/, "");
    const v = params[name] && params[name].value;
    let shown = String(v);
    if (kind === "height") shown = datesAvailable() ? `about ${dateOfHeight(v)} (block ${v})` : `block ${v}`;
    else if (kind === "erg") shown = `${v / 1e9} ERG`;
    else if (kind === "address") shown = shortAddr(v);
    else if (kind === "secret") shown = `(hash ${String(v).slice(0, 12)}… of the phrase you typed — keep the phrase)`;
    lines.push(`${label}: ${shown}`);
  }
  return lines.join("\n");
}

/// Storage rent, in words a non-technical user can act on.
function rentSentence(r, { forBurn = false, withHeight = true, network = null } = {}) {
  const erg = (r.feeNanoerg / 1e9).toFixed(3);
  const years = (r.periodBlocks * BLOCK_SECONDS / 86400 / 365.25).toFixed(1);
  let s = `Every box on Ergo pays storage rent: about every ${years} years a miner may take a fee of roughly ${erg} ERG from a box under this contract (based on its size), leaving the rest locked exactly as before. A box holding less than the fee is taken entirely, tokens included, so keep more than ${erg} ERG in it${forBurn ? "" : " if it must survive"}.`;
  if (forBurn) s += " For a burn address that means: ERG above the fee stays locked for decades; tokens in a box with little ERG will eventually be swept by a miner, not destroyed.";
  if (withHeight && r.nextCollectionHeight) {
    // A date is only honest for the network whose height we observed.
    const canDate = chainHeight != null && network != null && network === chainNetwork;
    s += canDate ? ` This box's first rent collection can happen at block ${r.nextCollectionHeight} (about ${dateOfHeight(r.nextCollectionHeight)}).` : ` This box's first rent collection can happen at block ${r.nextCollectionHeight}.`;
  }
  return s;
}

function renderQr(text) {
  const el = $("build-qr");
  el.textContent = "";
  try {
    const qr = qrcode(0, "M");
    qr.addData(text);
    qr.make();
    el.innerHTML = qr.createSvgTag({ cellSize: 3, margin: 2, scalable: true });
  } catch (e) { el.textContent = ""; }
}

$("wizard").addEventListener("submit", async (e) => {
  e.preventDefault();
  if (!recipe) return;
  const status = $("build-status");
  const { params, error } = wizardParams();
  if (error) { status.textContent = error; status.hidden = false; return; }
  status.textContent = "Creating…"; status.hidden = false;
  $("build-create").disabled = true;
  try {
    const network = $("build-network").value;
    const res = await fetch("/api/v1/compile", {
      method: "POST", headers: { "content-type": "application/json" },
      body: JSON.stringify({ source: recipe.source, network, params }),
    });
    const body = await res.json();
    if (!res.ok) {
      const m = (body.error && body.error.message) || String(res.status);
      status.textContent = /SigmaProp|address/i.test(m) ? "One of the addresses is not valid. Check it against your wallet." : `Something went wrong: ${m}`;
      return;
    }
    built = { ...body, params, network };
    renderChecks(null);
    $("build-rent").textContent = rentSentence(body.rent, { forBurn: recipe.name === "burn", withHeight: false });
    $("build-summary").textContent = describeBuild(params);
    $("build-address").textContent = body.p2s;
    $("build-tree").textContent = body.treeHex;
    renderQr(body.p2s);
    $("build-hunt").textContent = "Checking who can spend it…";
    status.hidden = true;
    buildStep(3);
    const hunt = await (await fetch("/api/v1/hunt", {
      method: "POST", headers: { "content-type": "application/json" },
      body: JSON.stringify({ input: body.treeHex, network }),
    })).json();
    // Recipes whose spending story the six probes cannot see (they carry no
    // secret and no oracle box) get their own sentence.
    const special = {
      "bounty": "Anyone who knows the secret phrase can claim this before the deadline — no key needed. After the deadline, only the funder can take it back.",
      "price-gate": "Only the owner can spend, and only in a transaction that includes the oracle's box showing a price at or above the floor.",
    }[recipe.name];
    $("build-hunt").textContent = special || {
      requiresProof: "Only the people you named can spend from this address, and only under the rules above. Nobody else can.",
      spendableByAnyone: "Warning: anyone could spend from this address as it stands. Check your answers before sending anything.",
      movableByAnyone: "Anyone can move the funds, but only back into this same contract.",
      notUnderProbes: recipe.name === "burn" ? "No transaction can ever satisfy this contract; only storage rent (below) can ever move anything out of it." : "Nobody could spend it in our checks.",
    }[hunt.verdict] || "";
  } catch (err) {
    status.textContent = `Something went wrong: ${err}`;
  } finally {
    $("build-create").disabled = false;
  }
});

$("build-back").addEventListener("click", () => buildStep(1));
$("build-again").addEventListener("click", () => { built = null; buildStep(1); });
$("build-network").addEventListener("change", () => { if (recipe) startRecipe(recipe); });
$("build-open-write").addEventListener("click", () => {
  if (!recipe) return;
  setEditorValue(recipe.source);
  $("params-rows").textContent = "";
  const { params } = wizardParams();
  renderParams(recipe.params.map((p) => ({ name: p.name, typeHint: p.typeHint, default: params && params[p.name] != null ? String(params[p.name].value) : p.default })));
  $("write-network").value = $("build-network").value;
  setMode("write");
});
$("build-copy").addEventListener("click", () => copyText($("build-address").textContent, "Address copied."));
$("build-share").addEventListener("click", () => {
  if (!built) return;
  const state = { s: recipe.source, p: built.params, n: built.network };
  const bytes = new TextEncoder().encode(JSON.stringify(state));
  let bin = ""; for (const b of bytes) bin += String.fromCharCode(b);
  const frag = btoa(bin).replace(/\+/g, "-").replace(/\//g, "_").replace(/=+$/, "");
  copyText(`${location.origin}${location.pathname}#s=${frag}`, "Link copied. It carries your answers; nothing is stored on the server.");
});
$("build-project").addEventListener("click", () => {
  if (!built) return;
  if (composedContract && recipe && recipe.name === "custom" && composedContract.suite) {
    $("tests").value = JSON.stringify(composedContract.suite.scenarios, null, 2);
  }
  downloadProject(recipe.source, built.params, built.network, `${recipe.doc ? recipe.doc.name : "contract"}`);
});

// ── composer: ways to spend → spec → compose → compile ───────────────────

const WHO_KINDS = [
  ["anyOf", "one person (or any of several)"],
  ["allOf", "all of these people together"],
  ["kOf", "some of these people (k of n)"],
  ["anyOne", "anyone — no signature needed"],
];
// The condition catalogue: what a script can see about the spending
// transaction, in plain words. Each kind renders its own small form and
// reads back one composer condition (plus the typed values it needs).
const COND_GROUPS = [
  ["When", [
    ["after", "only from a date"],
    ["before", "only until a date"],
    ["boxAge", "only after the funds have sat here for a while"],
    ["afterTime", "only from a clock time (block timestamp)"],
    ["beforeTime", "only until a clock time (block timestamp)"],
  ]],
  ["Payments", [
    ["payTo", "must pay someone"],
    ["sumPaidTo", "must pay someone in total, across all outputs"],
    ["keepHere", "must keep funds in this contract"],
    ["keepShare", "must keep a percentage of the funds in this contract"],
  ]],
  ["Tokens", [
    ["tokenGated", "spender must hold a token (membership)"],
    ["sendToken", "must send a token to someone"],
    ["keepTokens", "this box's tokens must stay with the funds kept here"],
    ["noTokensOut", "no output may carry tokens"],
  ]],
  ["Outside information (data inputs)", [
    ["oracleAbove", "only while an oracle price is at or above a floor"],
    ["oracleBelow", "only while an oracle price is at or below a ceiling"],
    ["dataToken", "a data input must carry a token"],
  ]],
  ["Records (registers)", [
    ["selfReg", "this box's register must hold a value"],
    ["stampHeight", "the funds kept here must record the current block height"],
    ["carryReg", "the funds kept here must carry over a register unchanged"],
  ]],
  ["Secrets, attached values, miner", [
    ["hashPreimage", "spender must reveal a secret phrase"],
    ["varEquals", "spender must attach a specific number"],
    ["minerIs", "only a specific miner may include the transaction"],
  ]],
  ["Shape of the transaction", [
    ["inputCount", "exactly this many inputs"],
    ["outputCount", "exactly this many outputs"],
    ["boxRule", "a rule on any box (advanced)"],
  ]],
];
const COND_LABELS = Object.fromEntries(COND_GROUPS.flatMap(([, ks]) => ks));
const REG_TYPES = ["Int", "Long", "Boolean", "Coll[Byte]"];
const REG_OPS = [["eq", "equals"], ["gte", "is at least"], ["lte", "is at most"], ["ne", "is not"]];

/// Field builders per kind. `mk(cls, label, attrs)` adds a labelled input;
/// `sel(cls, label, options)` a labelled select; `dates` says whether the
/// chain height is known (dates instead of heights).
const COND_FIELDS = {
  after: (mk, sel, dates) => mk("c-height", dates ? "From when?" : "From which block height?", dates ? { type: "datetime-local" } : { type: "number", min: "1" }),
  before: (mk, sel, dates) => mk("c-height", dates ? "Until when?" : "Until which block height?", dates ? { type: "datetime-local" } : { type: "number", min: "1" }),
  boxAge: (mk) => mk("c-days", "For how many days? (about 720 blocks a day)", { type: "number", min: "0", step: "0.1" }),
  afterTime: (mk) => mk("c-time", "From when? (the block's own clock, which miners set)", { type: "datetime-local" }),
  beforeTime: (mk) => mk("c-time", "Until when? (the block's own clock, which miners set)", { type: "datetime-local" }),
  payTo: (mk) => { mk("c-key", "Who must be paid? (address)", { spellcheck: false }); mk("c-erg", "At least how much, in ERG?", { type: "number", step: "0.000000001" }); },
  sumPaidTo: (mk) => { mk("c-key", "Who must be paid? (address)", { spellcheck: false }); mk("c-erg", "At least how much in total, in ERG?", { type: "number", step: "0.000000001" }); },
  keepHere: (mk) => mk("c-erg", "At least how much must stay, in ERG?", { type: "number", step: "0.000000001" }),
  keepShare: (mk) => mk("c-pct", "At least what percentage must stay?", { type: "number", min: "0", max: "100" }),
  tokenGated: (mk) => mk("c-token", "Token id the spender must hold (64 hex characters)", { spellcheck: false }),
  sendToken: (mk) => { mk("c-key", "Who receives the token? (address)", { spellcheck: false }); mk("c-token", "Token id (64 hex characters)", { spellcheck: false }); mk("c-num", "At least how many? (smallest units)", { type: "number", min: "1", value: "1" }); },
  keepTokens: () => {},
  noTokensOut: () => {},
  oracleAbove: (mk) => { mk("c-token", "Oracle token id (64 hex characters)", { spellcheck: false }); mk("c-num", "Minimum price, in the oracle's units", { type: "number" }); },
  oracleBelow: (mk) => { mk("c-token", "Oracle token id (64 hex characters)", { spellcheck: false }); mk("c-num", "Maximum price, in the oracle's units", { type: "number" }); },
  dataToken: (mk) => mk("c-token", "Token id the data input must carry (64 hex characters)", { spellcheck: false }),
  selfReg: (mk, sel) => regFields(mk, sel, true),
  stampHeight: (mk, sel) => sel("c-reg", "Which register?", ["R4", "R5", "R6", "R7", "R8", "R9"]),
  carryReg: (mk, sel) => { sel("c-reg", "Which register?", ["R4", "R5", "R6", "R7", "R8", "R9"]); sel("c-type", "What kind of value is in it?", REG_TYPES); },
  hashPreimage: (mk) => mk("c-secret", "The secret phrase (hashed here in your browser; only the hash goes on chain)", { autocomplete: "off" }),
  varEquals: (mk) => { mk("c-var", "Variable number (0–9)", { type: "number", min: "0", max: "9", value: "1" }); mk("c-num", "The number the spender must attach", { type: "number" }); },
  minerIs: (mk) => mk("c-hex", "Miner public key (66 hex characters)", { spellcheck: false }),
  inputCount: (mk) => mk("c-num", "How many inputs, counting this box?", { type: "number", min: "1", value: "1" }),
  outputCount: (mk) => mk("c-num", "How many outputs?", { type: "number", min: "1", value: "2" }),
  boxRule: (mk, sel) => {
    sel("c-which", "Which box?", [["output", "an output"], ["input", "an input"], ["dataInput", "a data input"], ["self", "this box"]]);
    mk("c-index", "Which one? (a number from 0, \"any\", or \"all\")", { value: "any", spellcheck: false });
    sel("c-script", "Its script must be", [["", "anything"], ["self", "this same contract"], ["key", "an address:"]]);
    mk("c-key", "Address", { spellcheck: false });
    mk("c-erg", "Value at least, in ERG (optional)", { type: "number", step: "0.000000001" });
    mk("c-pct", "Value at least this percentage of this box's value (optional)", { type: "number", min: "0", max: "100" });
    mk("c-token", "Must carry token id (optional, 64 hex characters)", { spellcheck: false });
    mk("c-num", "…at least this many (optional)", { type: "number", min: "1" });
    sel("c-tokens", "Tokens", [["", "no rule"], ["none", "must carry no tokens"], ["self", "must carry exactly this box's tokens"]]);
    sel("c-reg", "Register rule (optional)", [["", "none"], "R4", "R5", "R6", "R7", "R8", "R9"]);
    sel("c-type", "Register type", REG_TYPES);
    sel("c-op", "Register comparison", [...REG_OPS, ["eqHeight", "equals the current height (Int)"], ["eqSelf", "equals this box's same register"]]);
    mk("c-val", "Register value", { spellcheck: false });
  },
};
function regFields(mk, sel, withOp) {
  sel("c-reg", "Which register?", ["R4", "R5", "R6", "R7", "R8", "R9"]);
  sel("c-type", "What kind of value?", REG_TYPES);
  if (withOp) sel("c-op", "How must it compare?", REG_OPS);
  mk("c-val", "Value (a number, true/false, or hex bytes)", { spellcheck: false });
}

function hexOf(bytes) { return [...bytes].map((b) => b.toString(16).padStart(2, "0")).join(""); }

/// Readers per kind: `q(cls)` is the row's input; `c` the read context.
const COND_READ = {
  after: (q, c) => ({ after: c.set("after", "Int", c.heightOf(q("c-height"))) }),
  before: (q, c) => ({ before: c.set("before", "Int", c.heightOf(q("c-height"))) }),
  boxAge: (q, c) => { const d = Number(q("c-days").value); if (!(q("c-days").value.trim() && d >= 0)) throw new Error("How many days the funds must sit is missing."); return { boxAge: c.set("age", "Int", Math.ceil(d * 720)) }; },
  afterTime: (q, c) => ({ afterTime: c.set("fromTime", "Long", String(c.timeOf(q("c-time")))) }),
  beforeTime: (q, c) => ({ beforeTime: c.set("untilTime", "Long", String(c.timeOf(q("c-time")))) }),
  payTo: (q, c) => ({ payTo: { key: c.addKey(q("c-key").value), amount: c.set("amount", "Long", c.erg(q("c-erg"), "A payment amount in ERG is missing.")) } }),
  sumPaidTo: (q, c) => ({ sumPaidTo: { key: c.addKey(q("c-key").value), atLeast: c.set("total", "Long", c.erg(q("c-erg"), "A total amount in ERG is missing.")) } }),
  keepHere: (q, c) => ({ box: { which: "output", index: 0, script: "self", valueAtLeast: c.set("keep", "Long", c.erg(q("c-erg"), "A keep amount in ERG is missing.", true)) } }),
  keepShare: (q, c) => ({ box: { which: "output", index: 0, script: "self", valueAtLeastShare: { percent: c.set("keepPct", "Long", c.pct(q("c-pct"))) } } }),
  tokenGated: (q, c) => ({ tokenGated: { tokenId: c.set("memberToken", "Coll[Byte]", c.tokenId(q("c-token"))) } }),
  sendToken: (q, c) => ({ box: { which: "output", index: "any", script: { key: c.addKey(q("c-key").value) }, token: { id: c.set("token", "Coll[Byte]", c.tokenId(q("c-token"))), atLeast: c.set("tokenAmount", "Long", c.whole(q("c-num"), 1)) } } }),
  keepTokens: () => ({ box: { which: "output", index: 0, script: "self", keepsSelfTokens: true } }),
  noTokensOut: () => ({ box: { which: "output", index: "all", noTokens: true } }),
  oracleAbove: (q, c) => ({ oracleAbove: { nft: c.set("oracle", "Coll[Byte]", c.tokenId(q("c-token"))), floor: c.set("floor", "Long", c.whole(q("c-num"))) } }),
  oracleBelow: (q, c) => ({ box: { which: "dataInput", index: 0, token: { id: c.set("oracle", "Coll[Byte]", c.tokenId(q("c-token"))) }, registers: [{ reg: "R4", type: "Long", op: "lte", value: c.set("ceiling", "Long", c.whole(q("c-num"))) }] } }),
  dataToken: (q, c) => ({ box: { which: "dataInput", index: "any", token: { id: c.set("dataToken", "Coll[Byte]", c.tokenId(q("c-token"))) } } }),
  selfReg: (q, c) => ({ box: { which: "self", registers: [c.regRule(q, q("c-op").value)] } }),
  stampHeight: (q) => ({ box: { which: "output", index: 0, script: "self", registers: [{ reg: q("c-reg").value, type: "Int", op: "eqHeight" }] } }),
  carryReg: (q) => ({ box: { which: "output", index: 0, script: "self", registers: [{ reg: q("c-reg").value, type: q("c-type").value, op: "eqSelf" }] } }),
  hashPreimage: (q, c) => {
    const s = q("c-secret").value; if (!s) throw new Error("The secret phrase is missing.");
    const bytes = new TextEncoder().encode(s); const idx = c.nextVar();
    c.witness[String(idx)] = { type: "Coll[Byte]", value: hexOf(bytes) };
    return { hashPreimage: { var: idx, hash: c.set("secretHash", "Coll[Byte]", window.blake2b.blake2bHex(bytes, null, 32)), algo: "blake2b256" } };
  },
  varEquals: (q, c) => ({ varEquals: { index: c.whole(q("c-var"), 0), type: "Int", value: c.set("attached", "Int", c.whole(q("c-num"))) } }),
  minerIs: (q, c) => { const h = q("c-hex").value.trim().toLowerCase(); if (!/^[0-9a-f]{66}$/.test(h)) throw new Error("A miner public key is 66 hex characters."); return { minerIs: c.set("miner", "Coll[Byte]", h) }; },
  inputCount: (q, c) => ({ inputCount: c.set("inputs", "Int", c.whole(q("c-num"), 1)) }),
  outputCount: (q, c) => ({ outputCount: c.set("outputs", "Int", c.whole(q("c-num"), 1)) }),
  boxRule: (q, c) => {
    const r = { which: q("c-which").value };
    const idx = q("c-index").value.trim().toLowerCase();
    if (r.which !== "self") { if (idx === "any" || idx === "all") r.index = idx; else if (/^\d+$/.test(idx)) r.index = Number(idx); else throw new Error("\"Which one?\" is a number, \"any\", or \"all\"."); }
    const sc = q("c-script").value;
    if (sc === "self") r.script = "self"; else if (sc === "key") r.script = { key: c.addKey(q("c-key").value) };
    if (q("c-erg").value.trim()) r.valueAtLeast = c.set("value", "Long", c.erg(q("c-erg"), "A value in ERG does not parse.", true));
    if (q("c-pct").value.trim()) r.valueAtLeastShare = { percent: c.set("share", "Long", c.pct(q("c-pct"))) };
    if (q("c-token").value.trim()) { r.token = { id: c.set("token", "Coll[Byte]", c.tokenId(q("c-token"))) }; if (q("c-num").value.trim()) r.token.atLeast = c.set("tokenAmount", "Long", c.whole(q("c-num"), 1)); }
    if (q("c-tokens").value === "none") r.noTokens = true;
    if (q("c-tokens").value === "self") r.keepsSelfTokens = true;
    if (q("c-reg").value) { const op = q("c-op").value; r.registers = [/^(eq|gte|lte|ne)$/.test(op) ? c.regRule(q, op) : { reg: q("c-reg").value, type: q("c-type").value, op }]; }
    return { box: r };
  },
};

/// Words for a composed condition (the engine's shape), for the summary.
function describeCond(c, values) {
  const v = (n) => values[n] ? values[n].value : n;
  const erg = (n) => `${Number(v(n)) / 1e9} ERG`;
  const tok = (n) => `token ${String(v(n)).slice(0, 8)}…`;
  if (c.after) return `from block ${v(c.after)}`;
  if (c.before) return `until block ${v(c.before)}`;
  if (c.afterTime) return `from ${new Date(Number(v(c.afterTime))).toLocaleString()} (block clock)`;
  if (c.beforeTime) return `until ${new Date(Number(v(c.beforeTime))).toLocaleString()} (block clock)`;
  if (c.boxAge) return `after the funds have sat here ${v(c.boxAge)} blocks`;
  if (c.inputCount) return `exactly ${v(c.inputCount)} input(s)`;
  if (c.outputCount) return `exactly ${v(c.outputCount)} output(s)`;
  if (c.payTo) return `paying ${shortAddr(v(c.payTo.key))} at least ${erg(c.payTo.amount)}`;
  if (c.sumPaidTo) return `paying ${shortAddr(v(c.sumPaidTo.key))} at least ${erg(c.sumPaidTo.atLeast)} in total`;
  if (c.keepHere) return `keeping at least ${erg(c.keepHere.atLeast)} here`;
  if (c.oracleAbove) return `oracle price ≥ ${v(c.oracleAbove.floor)}`;
  if (c.tokenGated) return `the spender holds ${tok(c.tokenGated.tokenId)}`;
  if (c.hashPreimage) return "the spender reveals the secret phrase";
  if (c.varEquals) return `the spender attaches ${v(c.varEquals.value)} as variable ${c.varEquals.index}`;
  if (c.minerIs) return `mined by ${String(v(c.minerIs)).slice(0, 10)}…`;
  if (c.box) {
    const r = c.box;
    const kept = r.which === "output" && r.index === 0 && r.script === "self";
    const which = kept ? "the funds kept here" : r.which === "self" ? "this box" : r.index === "any" ? `some ${r.which === "dataInput" ? "data input" : r.which}` : r.index === "all" ? `every ${r.which === "dataInput" ? "data input" : r.which}` : `${r.which === "dataInput" ? "data input" : r.which} ${r.index}`;
    const parts = [];
    if (r.script === "self" && !kept) parts.push("stays under this contract"); else if (r.script && r.script.key) parts.push(`goes to ${shortAddr(v(r.script.key))}`);
    if (r.valueAtLeast) parts.push(`holds at least ${erg(r.valueAtLeast)}`);
    if (r.valueAtLeastShare) parts.push(`holds at least ${v(r.valueAtLeastShare.percent)}% of this box's value`);
    if (r.token) parts.push(`carries ${tok(r.token.id)}${r.token.atLeast ? ` ×${v(r.token.atLeast)}+` : ""}`);
    if (r.noTokens) parts.push("carries no tokens");
    if (r.keepsSelfTokens) parts.push("carries exactly this box's tokens");
    for (const rr of r.registers || []) {
      const op = { eq: "=", ne: "≠", gte: "≥", lte: "≤" }[rr.op];
      parts.push(rr.op === "eqHeight" ? `${rr.reg} records the current height` : rr.op === "eqSelf" ? `${rr.reg} carries over this box's ${rr.reg}` : `${rr.reg} ${op} ${v(rr.value)}`);
    }
    return parts.length ? `${which} ${parts.join(" and ")}` : which;
  }
  return "";
}

let composedContract = null; // the spec-derived source + answers, for step 3

function startComposer() {
  recipe = null; built = null;
  $("paths").textContent = "";
  addPath();
  $("compose-status").hidden = true;
  buildStepEl("build-compose");
}

function addPath() {
  const n = $("paths").children.length + 1;
  const box = document.createElement("div");
  box.className = "path";
  box.innerHTML = `
    <div class="path-head"><strong>Way to spend ${n}</strong> <button type="button" class="secondary tiny path-remove">remove</button></div>
    <div class="field"><label>Who may spend this way?</label><select class="who-kind"></select></div>
    <div class="who-keys"></div>
    <div class="field"><label>Under what conditions?</label><div class="conds"></div>
      <button type="button" class="secondary tiny cond-add">+ add a condition</button></div>`;
  const whoSel = box.querySelector(".who-kind");
  for (const [v, l] of WHO_KINDS) { const o = document.createElement("option"); o.value = v; o.textContent = l; whoSel.appendChild(o); }
  const renderKeys = () => {
    const keys = box.querySelector(".who-keys");
    const kind = whoSel.value;
    keys.textContent = "";
    if (kind === "anyOne") return;
    const count = keys.dataset.count ? Number(keys.dataset.count) : (kind === "anyOf" ? 1 : 2);
    keys.dataset.count = String(count);
    if (kind === "kOf") {
      const f = document.createElement("div"); f.className = "field";
      f.innerHTML = `<label>How many of them must agree?</label><input class="kof" type="number" min="1" value="2">`;
      keys.appendChild(f);
    }
    for (let i = 0; i < count; i++) {
      const f = document.createElement("div"); f.className = "field";
      f.innerHTML = `<label>Address ${i + 1}</label><input class="key" placeholder="${$("compose-network").value === "testnet" ? "3…" : "9…"}" spellcheck="false"><span class="problem" hidden></span>`;
      const inp = f.querySelector("input");
      inp.addEventListener("input", () => { const m = fieldProblem("address", inp.value, $("compose-network").value); const pr = f.querySelector(".problem"); pr.textContent = m; pr.hidden = !m; inp.classList.toggle("invalid", !!m); });
      keys.appendChild(f);
    }
    const more = document.createElement("button"); more.type = "button"; more.className = "secondary tiny";
    more.textContent = "+ another address";
    more.addEventListener("click", () => { keys.dataset.count = String(count + 1); renderKeys(); });
    keys.appendChild(more);
  };
  whoSel.addEventListener("change", () => { box.querySelector(".who-keys").dataset.count = ""; renderKeys(); });
  renderKeys();
  box.querySelector(".cond-add").addEventListener("click", () => addCond(box.querySelector(".conds")));
  box.querySelector(".path-remove").addEventListener("click", () => { box.remove(); renumberPaths(); });
  $("paths").appendChild(box);
}

function renumberPaths() {
  [...$("paths").children].forEach((b, i) => { b.querySelector(".path-head strong").textContent = `Way to spend ${i + 1}`; });
}

function addCond(container) {
  const row = document.createElement("div");
  row.className = "cond";
  const sel = document.createElement("select"); sel.className = "cond-kind";
  for (const [g, kinds] of COND_GROUPS) {
    const og = document.createElement("optgroup"); og.label = g;
    for (const [v, l] of kinds) { const o = document.createElement("option"); o.value = v; o.textContent = l; og.appendChild(o); }
    sel.appendChild(og);
  }
  const inputs = document.createElement("div"); inputs.className = "cond-inputs";
  const rm = document.createElement("button"); rm.type = "button"; rm.className = "secondary tiny"; rm.textContent = "remove";
  rm.addEventListener("click", () => row.remove());
  const render = () => {
    inputs.textContent = "";
    const mk = (cls, label, attrs) => {
      const f = document.createElement("div"); f.className = "field";
      const l = document.createElement("label"); l.textContent = label;
      const i = document.createElement("input"); i.className = cls; Object.assign(i, attrs || {});
      f.append(l, i); inputs.appendChild(f); return i;
    };
    const sel2 = (cls, label, options) => {
      const f = document.createElement("div"); f.className = "field";
      const l = document.createElement("label"); l.textContent = label;
      const el = document.createElement("select"); el.className = cls;
      for (const opt of options) { const [v, t] = Array.isArray(opt) ? opt : [opt, opt]; const o = document.createElement("option"); o.value = v; o.textContent = t; el.appendChild(o); }
      f.append(l, el); inputs.appendChild(f); return el;
    };
    COND_FIELDS[sel.value](mk, sel2, datesAvailableFor($("compose-network").value));
  };
  sel.addEventListener("change", render);
  render();
  row.append(sel, inputs, rm);
  container.appendChild(row);
}

function datesAvailableFor(network) { return chainHeight != null && chainNetwork === network; }

/// Read the composer UI into a spec plus typed values. Parameter names are
/// generated (`key1`, `after1`, …) so the source stays readable.
function readComposer() {
  const network = $("compose-network").value;
  const spec = { paths: [], witness: {} };
  const values = {};
  let keyN = 0, condN = 0, varN = 0;
  const addKey = (addr) => { const p = fieldProblem("address", addr, network); if (!addr.trim()) throw new Error("An address is missing."); if (p) throw new Error(p); keyN++; const name = `key${keyN}`; values[name] = { type: "SigmaProp", value: addr.trim() }; return name; };
  const heightOf = (inp) => {
    const raw = inp.value.trim(); if (!raw) throw new Error("A date or height is missing.");
    if (inp.type === "datetime-local") { const t = new Date(raw).getTime(); if (Number.isNaN(t)) throw new Error("That date does not parse."); const now = heightNow(); const h = now + Math.ceil((t - Date.now()) / 1000 / BLOCK_SECONDS); if (h <= now) throw new Error("Dates must be in the future."); return h; }
    const h = Number(raw); if (!Number.isInteger(h) || h < 1) throw new Error("A block height is a whole number."); return h;
  };
  const usedVars = new Set([...$("paths").querySelectorAll(".cond")].filter((r) => r.querySelector(".cond-kind").value === "varEquals").map((r) => Number(r.querySelector(".c-var").value)));
  const ctx = {
    addKey, heightOf, witness: spec.witness,
    set: (prefix, type, value) => { const name = `${prefix}${condN}`; values[name] = { type, value }; return name; },
    timeOf: (inp) => { const t = new Date(inp.value).getTime(); if (!inp.value || Number.isNaN(t)) throw new Error("A time is missing or does not parse."); return t; },
    erg: (inp, msg, zeroOk) => { const raw = inp.value.trim(); const e = Number(raw); if (!raw || Number.isNaN(e) || (zeroOk ? e < 0 : e <= 0)) throw new Error(msg); return Math.round(e * 1e9); },
    pct: (inp) => { const n = Number(inp.value); if (!(inp.value.trim() && n >= 0 && n <= 100)) throw new Error("A percentage is between 0 and 100."); return Math.round(n); },
    tokenId: (inp) => { const t = inp.value.trim().toLowerCase(); if (!/^[0-9a-f]{64}$/.test(t)) throw new Error("A token id is 64 hex characters."); return t; },
    whole: (inp, min) => { const raw = inp.value.trim(); const n = Number(raw); if (!raw || !Number.isInteger(n) || (min != null && n < min)) throw new Error(`"${inp.previousElementSibling ? inp.previousElementSibling.textContent : "A number"}" needs a whole number${min != null ? ` of at least ${min}` : ""}.`); return n; },
    // Secrets take the variable numbers the user has not already given to
    // "attach a specific number" conditions.
    nextVar: () => { while (usedVars.has(varN)) varN++; return varN++; },
    regRule: (q, op) => {
      const type = q("c-type").value; const raw = q("c-val").value.trim();
      let value;
      if (type === "Boolean") { if (!/^(true|false)$/i.test(raw)) throw new Error("A Boolean register value is true or false."); value = raw.toLowerCase() === "true"; }
      else if (type === "Coll[Byte]") { if (!/^([0-9a-fA-F]{2})+$/.test(raw)) throw new Error("A Coll[Byte] register value is hex bytes."); value = raw.toLowerCase(); }
      else { if (!/^-?\d+$/.test(raw)) throw new Error(`An ${type} register value is a whole number.`); value = raw; }
      return { reg: q("c-reg").value, type, op, value: ctx.set("regValue", type, value) };
    },
  };
  for (const box of $("paths").children) {
    const kind = box.querySelector(".who-kind").value;
    const keys = [...box.querySelectorAll(".who-keys .key")].map((i) => addKey(i.value));
    let who;
    if (kind === "anyOne") who = { anyOne: true };
    else if (kind === "anyOf") who = { anyOf: keys };
    else if (kind === "allOf") who = { allOf: keys };
    else { const k = Number(box.querySelector(".kof").value); if (!(k >= 1 && k <= keys.length)) throw new Error(`"How many must agree" must be between 1 and ${keys.length}.`); who = { kOf: k, keys }; }
    const conditions = [];
    const rows = [...box.querySelectorAll(".cond")];
    // "The funds kept here" is always output 0; payments take the slots after it.
    const keepsFirst = rows.some((r) => /^(keepHere|keepShare|keepTokens|stampHeight|carryReg)$/.test(r.querySelector(".cond-kind").value));
    let paySlot = keepsFirst ? 1 : 0;
    for (const row of rows) {
      condN++;
      const ck = row.querySelector(".cond-kind").value;
      const q = (cls) => row.querySelector(`.${cls}`);
      let cond;
      try { cond = COND_READ[ck](q, ctx); } catch (e) { throw new Error(`Way ${spec.paths.length + 1}, "${COND_LABELS[ck]}": ${e.message}`); }
      if (cond.payTo) { cond = { box: { which: "output", index: paySlot++, script: { key: cond.payTo.key }, valueAtLeast: cond.payTo.amount } }; }
      conditions.push(cond);
    }
    spec.paths.push({ name: `way ${spec.paths.length + 1}`, who, conditions });
  }
  if (!Object.keys(spec.witness).length) delete spec.witness;
  return { spec, values, network };
}

$("path-add").addEventListener("click", addPath);
$("compose-back").addEventListener("click", () => buildStep(1));
$("compose-network").addEventListener("change", () => { for (const r of $("paths").querySelectorAll(".cond")) r.querySelector(".cond-kind").dispatchEvent(new Event("change")); });
$("compose-create").addEventListener("click", async () => {
  const status = $("compose-status");
  let read;
  try { read = readComposer(); } catch (e) { status.textContent = e.message; status.hidden = false; return; }
  status.textContent = "Creating and checking…"; status.hidden = false;
  $("compose-create").disabled = true;
  try {
    const cres = await fetch("/api/v1/compose", { method: "POST", headers: { "content-type": "application/json" }, body: JSON.stringify({ spec: read.spec, params: read.values, run: true }) });
    const composed = await cres.json();
    if (!cres.ok) { status.textContent = `Could not build that: ${(composed.error && composed.error.message) || cres.status}`; return; }
    const res = await fetch("/api/v1/compile", { method: "POST", headers: { "content-type": "application/json" }, body: JSON.stringify({ source: composed.source, network: read.network, params: read.values }) });
    const body = await res.json();
    if (!res.ok) { status.textContent = `Could not create the contract: ${(body.error && body.error.message) || res.status}`; return; }
    composedContract = { source: composed.source, params: read.values, network: read.network, suite: composed.suite };
    built = { ...body, params: read.values, network: read.network };
    recipe = { name: "custom", source: composed.source, params: composed.params, doc: { name: "custom", description: "" } };
    $("build-rent").textContent = rentSentence(body.rent, { withHeight: false });
    $("build-summary").textContent = read.spec.paths.map((p, i) => `Way ${i + 1}: ${describeWho(p.who, read.values)}${p.conditions.length ? ", " + p.conditions.map((c) => describeCond(c, read.values)).join(", ") : ""}.`).join("\n");
    $("build-address").textContent = body.p2s;
    $("build-tree").textContent = body.treeHex;
    renderQr(body.p2s);
    renderChecks(composed.results);
    $("build-hunt").textContent = body.findings.length ? `Note: ${body.findings.length} finding(s) in the compiled code — open in Write to see them.` : "";
    status.hidden = true;
    buildStep(3);
  } catch (err) {
    status.textContent = `Something went wrong: ${err}`;
  } finally {
    $("compose-create").disabled = false;
  }
});

function describeWho(who, values) {
  const a = (k) => shortAddr(values[k] ? values[k].value : k);
  if (who.anyOne) return "anyone";
  if (who.anyOf) return who.anyOf.length === 1 ? a(who.anyOf[0]) : `any of ${who.anyOf.map(a).join(", ")}`;
  if (who.allOf) return `all of ${who.allOf.map(a).join(", ")}`;
  return `${who.kOf} of ${who.keys.map(a).join(", ")}`;
}
function renderChecks(results) {
  const box = $("build-checks");
  if (!results) { box.hidden = true; return; }
  const tb = $("build-checks-rows"); tb.textContent = "";
  for (const c of results.cases) {
    const tr = document.createElement("tr");
    tr.dataset.verdict = c.passed ? "ok" : "fail";
    const outcome = { pass: "spendable by anyone", needsProof: "needs the named signature(s)", fail: "refused", error: "refused (script error)" }[c.actual] || c.actual;
    for (const cell of [c.passed ? "✓" : "✗", c.name, outcome]) { const td = document.createElement("td"); td.textContent = cell; tr.appendChild(td); }
    tb.appendChild(tr);
  }
  box.hidden = false;
}

// ── files: open .es / project files, save a project zip ──────────────────

$("open-file").addEventListener("click", () => $("file-input").click());
$("file-input").addEventListener("change", async (e) => {
  for (const f of e.target.files) {
    const text = await f.text();
    if (f.name.endsWith(".es")) { setEditorValue(text); $("params-rows").textContent = ""; renderParams(scanLocal(text)); }
    else if (/params\.json$/.test(f.name)) {
      try {
        const p = JSON.parse(text);
        $("params-rows").textContent = "";
        renderParams(Object.entries(p).map(([name, tv]) => ({ name, typeHint: tv.type, default: typeof tv.value === "object" ? JSON.stringify(tv.value) : String(tv.value) })));
      } catch (err) { /* ignore a bad params file */ }
    }
    else if (/test\.json$/.test(f.name)) {
      try {
        const suite = JSON.parse(text);
        if (Array.isArray(suite)) $("tests").value = JSON.stringify(suite, null, 2);
        else if (suite && Array.isArray(suite.scenarios)) {
          $("tests").value = JSON.stringify(suite.scenarios, null, 2);
          if (typeof suite.source === "string") setEditorValue(suite.source);
          if (suite.params) { $("params-rows").textContent = ""; renderParams(Object.entries(suite.params).map(([name, tv]) => ({ name, typeHint: tv.type, default: String(tv.value) }))); }
        }
      } catch (err) { /* ignore */ }
    }
  }
  e.target.value = "";
  setMode("write");
});

/// Cheap client-side scan of `$names` so an opened file gets its form
/// before the first compile (the server's scan is authoritative).
function scanLocal(src) {
  const seen = new Set(), out = [];
  const hints = {};
  for (const m of src.matchAll(/\/\/\s*\$([A-Za-z_][A-Za-z0-9_]*)\s*:\s*([A-Za-z\[\]]+)/g)) hints[m[1]] = m[2];
  const code = src.replace(/\/\*[\s\S]*?\*\//g, " ").replace(/\/\/.*$/gm, " ");
  for (const m of code.matchAll(/\$([A-Za-z_][A-Za-z0-9_]*)/g)) {
    if (!seen.has(m[1])) { seen.add(m[1]); out.push({ name: m[1], typeHint: hints[m[1]] || null }); }
  }
  return out;
}

$("save-project").addEventListener("click", () => {
  downloadProject(editorValue(), collectParams(), $("write-network").value, "contract");
});

/// contract.es + params.json + contract.test.json, zipped (STORE, no
/// compression — a few KB) so the whole project is one download that the
/// CLI runs unchanged: `ergo-es test contract.test.json`.
function downloadProject(source, params, network, baseName) {
  let scenarios = [];
  try { const t = JSON.parse($("tests").value); if (Array.isArray(t)) scenarios = t; } catch (e) { /* none */ }
  const files = [
    ["contract.es", source],
    ["params.json", JSON.stringify(params, null, 2) + "\n"],
    ["contract.test.json", JSON.stringify({ source, params, network, scenarios }, null, 2) + "\n"],
    ["README.md", `# ${baseName}\n\nCompiled and tested with ergo-forge.\n\n    ergo-es compile contract.es --params params.json --network ${network}\n    ergo-es test contract.test.json\n`],
  ];
  const blob = new Blob([zipStore(files)], { type: "application/zip" });
  const a = document.createElement("a");
  a.href = URL.createObjectURL(blob);
  a.download = `${baseName}.zip`;
  document.body.appendChild(a); a.click(); a.remove();
  setTimeout(() => URL.revokeObjectURL(a.href), 1000);
}

/// Minimal ZIP writer (method 0 = STORE). Enough for a handful of text files.
function zipStore(files) {
  const enc = new TextEncoder();
  const table = (() => { const t = new Uint32Array(256); for (let n = 0; n < 256; n++) { let c = n; for (let k = 0; k < 8; k++) c = c & 1 ? 0xedb88320 ^ (c >>> 1) : c >>> 1; t[n] = c >>> 0; } return t; })();
  const crc32 = (buf) => { let c = 0xffffffff; for (const b of buf) c = table[(c ^ b) & 0xff] ^ (c >>> 8); return (c ^ 0xffffffff) >>> 0; };
  const parts = [], central = [];
  let offset = 0;
  const u16 = (n) => [n & 0xff, (n >>> 8) & 0xff];
  const u32 = (n) => [n & 0xff, (n >>> 8) & 0xff, (n >>> 16) & 0xff, (n >>> 24) & 0xff];
  for (const [name, text] of files) {
    const nameB = enc.encode(name), data = enc.encode(text), crc = crc32(data);
    const local = new Uint8Array([...u32(0x04034b50), ...u16(20), ...u16(0), ...u16(0), ...u16(0), ...u16(0), ...u32(crc), ...u32(data.length), ...u32(data.length), ...u16(nameB.length), ...u16(0), ...nameB]);
    parts.push(local, data);
    central.push(new Uint8Array([...u32(0x02014b50), ...u16(20), ...u16(20), ...u16(0), ...u16(0), ...u16(0), ...u16(0), ...u32(crc), ...u32(data.length), ...u32(data.length), ...u16(nameB.length), ...u16(0), ...u16(0), ...u16(0), ...u16(0), ...u32(0), ...u32(offset), ...nameB]));
    offset += local.length + data.length;
  }
  const cdSize = central.reduce((n, c) => n + c.length, 0);
  const end = new Uint8Array([...u32(0x06054b50), ...u16(0), ...u16(0), ...u16(files.length), ...u16(files.length), ...u32(cdSize), ...u32(offset), ...u16(0)]);
  const total = parts.concat(central, [end]);
  const out = new Uint8Array(total.reduce((n, p) => n + p.length, 0));
  let pos = 0; for (const p of total) { out.set(p, pos); pos += p.length; }
  return out;
}

// ── export: share link, SDK snippets ─────────────────────────────────────

/// The editor state as a URL fragment: base64url of {source, params, network}.
function encodeShare() {
  const state = { s: editorValue(), p: collectParams(), n: $("write-network").value };
  const bytes = new TextEncoder().encode(JSON.stringify(state));
  let bin = ""; for (const b of bytes) bin += String.fromCharCode(b);
  return btoa(bin).replace(/\+/g, "-").replace(/\//g, "_").replace(/=+$/, "");
}
function decodeShare(frag) {
  const b64 = frag.replace(/-/g, "+").replace(/_/g, "/") + "=".repeat((4 - frag.length % 4) % 4);
  const bin = atob(b64);
  const bytes = Uint8Array.from(bin, (c) => c.charCodeAt(0));
  return JSON.parse(new TextDecoder().decode(bytes));
}

/// Apply a shared state: source, params (form rebuilt from the scan), network.
async function loadShared(frag) {
  let st;
  try { st = decodeShare(frag); } catch (e) { return; }
  if (typeof st.s !== "string") return;
  setEditorValue(st.s);
  if (st.n === "testnet" || st.n === "mainnet") $("write-network").value = st.n;
  $("params-rows").textContent = "";
  const needs = Object.entries(st.p || {}).map(([name, tv]) => ({ name, typeHint: tv.type, default: typeof tv.value === "object" ? JSON.stringify(tv.value) : String(tv.value) }));
  renderParams(needs);
  setMode("write");
}

async function copyText(text, okMessage) {
  const status = $("build").hidden ? $("export-status") : $("build-status");
  try { await navigator.clipboard.writeText(text); toast(okMessage); }
  catch (e) { status.textContent = text; status.hidden = false; }
}

/// A brief confirmation at the bottom of the screen.
function toast(msg) {
  let t = document.getElementById("toast");
  if (!t) { t = document.createElement("div"); t.id = "toast"; t.setAttribute("role", "status"); document.body.appendChild(t); }
  t.textContent = msg; t.classList.add("show");
  clearTimeout(t._h); t._h = setTimeout(() => t.classList.remove("show"), 2500);
}

$("share").addEventListener("click", () => {
  const url = `${location.origin}${location.pathname}#s=${encodeShare()}`;
  history.replaceState(null, "", `#s=${encodeShare()}`);
  copyText(url, "Share link copied — it carries the source, parameters and network in the URL fragment (nothing is stored server-side).");
});

/// Fleet SDK: the compiled tree as an ErgoTree with the address, plus the
/// parameters as named constants for reference.
$("export-fleet").addEventListener("click", () => {
  if (!lastCompiled) { copyText("", "Compile first."); return; }
  const c = lastCompiled;
  const params = collectParams();
  const consts = Object.entries(params).map(([k, v]) => `  // ${k}: ${v.type} = ${JSON.stringify(v.value)}`).join("\n");
  const code = `import { ErgoTree, ErgoAddress, Network } from "@fleet-sdk/core";

// Compiled by ergo-forge on the node's own compiler (byte-exact vs the reference).
// Source parameters:
${consts || "  // (none)"}
const TREE_HEX = "${c.treeHex}";

const tree = new ErgoTree(TREE_HEX);
const address = ErgoAddress.fromErgoTree(TREE_HEX, Network.${$("write-network").value === "testnet" ? "Testnet" : "Mainnet"});
// address.encode() === "${c.p2s}"

// Use it as an output's script:
// new OutputBuilder(value, address)
`;
  copyText(code, "Fleet SDK snippet copied.");
});

/// appkit (Scala/Java): the same tree via the address and the raw bytes.
$("export-appkit").addEventListener("click", () => {
  if (!lastCompiled) { copyText("", "Compile first."); return; }
  const c = lastCompiled;
  const params = collectParams();
  const consts = Object.entries(params).map(([k, v]) => `  // ${k}: ${v.type} = ${JSON.stringify(v.value)}`).join("\n");
  const code = `import org.ergoplatform.appkit._

// Compiled by ergo-forge on the node's own compiler (byte-exact vs the reference).
// Source parameters:
${consts || "  // (none)"}
val treeHex = "${c.treeHex}"
val contract: ErgoContract = ErgoTreeContract.fromErgoTree(
  JavaHelpers.decodeStringToBytes(treeHex), NetworkType.${$("write-network").value === "testnet" ? "TESTNET" : "MAINNET"}
)
// contract.toAddress.toString == "${c.p2s}"
`;
  copyText(code, "appkit snippet copied.");
});

let firstVisit = true;
try { firstVisit = !localStorage.getItem("ergo-forge-seen"); localStorage.setItem("ergo-forge-seen", "1"); } catch (e) { /* storage blocked: treat as a first visit */ }
if (location.hash.startsWith("#s=")) loadShared(location.hash.slice(3));
else if (location.hash === "#build" || firstVisit) setMode("build");

// ── validate a transaction ───────────────────────────────────────────────

$("validate-tx").addEventListener("click", async () => {
  const status = $("vtx-status");
  let req;
  try { req = JSON.parse($("txjson").value); } catch (e) { status.textContent = `JSON does not parse: ${e.message}`; status.hidden = false; return; }
  if (req && req.inputs && !req.tx) req = { tx: req };
  status.textContent = "Validating…"; status.hidden = false;
  $("vtx-result").hidden = true;
  try {
    const res = await fetch("/api/v1/validate-tx", {
      method: "POST", headers: { "content-type": "application/json" }, body: JSON.stringify(req),
    });
    const body = await res.json();
    if (!res.ok) { status.textContent = `Error: ${(body.error && body.error.message) || res.status}`; return; }
    const v = $("vtx-verdict");
    v.textContent = body.valid
      ? `Would validate — ${body.signaturesNeeded} signature(s) needed. ERG in ${body.ergIn}, out ${body.ergOut}, at height ${body.height}.`
      : `Would be rejected. ERG in ${body.ergIn}, out ${body.ergOut}, at height ${body.height}.`;
    v.className = `hunt-verdict ${body.valid ? "ok" : "bad"}`;
    const probs = $("vtx-problems"); probs.textContent = "";
    for (const p of body.problems) { const li = document.createElement("li"); li.textContent = p; probs.appendChild(li); }
    const tb = $("vtx-rows"); tb.textContent = "";
    for (const i of body.inputs) {
      const tr = document.createElement("tr");
      tr.dataset.verdict = i.verdict === "pass" ? "ok" : (i.verdict === "needsProof" ? "needsProof" : "fail");
      for (const cell of [String(i.index), i.address || i.boxId, i.verdict, i.error || (i.verdict === "needsProof" ? i.reducedTo : "")]) {
        const td = document.createElement("td"); td.textContent = cell; tr.appendChild(td);
      }
      tb.appendChild(tr);
    }
    status.hidden = true; $("vtx-result").hidden = false;
  } catch (e) { status.textContent = `Request failed: ${e}`; }
});

// ── contract tests ───────────────────────────────────────────────────────

let testsInFlight = false;

/// The suite the panel would run: the editor's contract + the scenarios.
function currentSuite() {
  let scenarios;
  try {
    scenarios = JSON.parse($("tests").value);
  } catch (e) {
    return { error: `Scenarios JSON does not parse: ${e.message}` };
  }
  if (!Array.isArray(scenarios)) return { error: "Scenarios must be a JSON array." };
  return {
    suite: {
      source: editorValue(),
      params: collectParams(),
      network: $("write-network").value,
      scenarios,
    },
  };
}

async function runTests() {
  if (testsInFlight) return;
  const status = $("tests-status");
  const { suite, error } = currentSuite();
  if (error) { status.textContent = error; status.hidden = false; return; }
  testsInFlight = true;
  $("run-tests").disabled = true;
  status.textContent = "Running…";
  status.hidden = false;
  $("tests-result").hidden = true;
  try {
    const res = await fetch("/api/v1/test", {
      method: "POST", headers: { "content-type": "application/json" },
      body: JSON.stringify(suite),
    });
    const body = await res.json();
    if (!res.ok) {
      status.textContent = `Error: ${(body.error && body.error.message) || res.status}`;
      return;
    }
    const sum = $("tests-summary");
    sum.textContent = `${body.passed} passed, ${body.failed} failed`;
    sum.className = `hunt-verdict ${body.failed ? "bad" : "ok"}`;
    const tb = $("tests-rows");
    tb.textContent = "";
    for (const c of body.cases) {
      const tr = document.createElement("tr");
      tr.dataset.verdict = c.passed ? "ok" : "fail";
      for (const cell of [c.passed ? "✓" : "✗", c.name, c.expected, c.actual, String(c.cost),
                          c.error ? c.error : (c.reducedTo && c.actual === "needsProof" ? c.reducedTo : "")]) {
        const td = document.createElement("td"); td.textContent = cell; tr.appendChild(td);
      }
      tb.appendChild(tr);
    }
    status.hidden = true;
    $("tests-result").hidden = false;
  } catch (e) {
    status.textContent = `Request failed: ${e}`;
  } finally {
    testsInFlight = false;
    $("run-tests").disabled = false;
  }
}

$("run-tests").addEventListener("click", runTests);
$("export-tests").addEventListener("click", async () => {
  const { suite, error } = currentSuite();
  const status = $("tests-status");
  if (error) { status.textContent = error; status.hidden = false; return; }
  const text = JSON.stringify(suite, null, 2);
  try {
    await navigator.clipboard.writeText(text);
    status.textContent = "Copied contract.test.json to the clipboard — run it with: ergo-es test contract.test.json";
  } catch (e) {
    status.textContent = text;
  }
  status.hidden = false;
});

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

// ── chain lookups (only when the instance is configured with an explorer) ──

let fetchedBoxes = [];

async function loadConfig() {
  try {
    const cfg = await (await fetch("/api/v1/config")).json();
    if (cfg.height) { chainHeight = cfg.height; chainHeightAt = Date.now(); chainNetwork = cfg.network || "mainnet"; }
    if (cfg.explorer) {
      $("chain-panel").hidden = false;
      $("footer-note").textContent =
        "source, findings and verdicts are computed locally; this instance fetches box data from a configured explorer when you ask it to.";
    }
  } catch (e) { /* stay in the no-outbound mode */ }
}

function useFetchedBox(i) {
  const b = fetchedBoxes[i];
  if (!b) return;
  const { boxId, ...scenarioBox } = b;
  $("self-box").value = JSON.stringify(scenarioBox);
  $("self-box").closest("details").open = true;
  const input = $("input").value.trim();
  if (input) huntFor(input, $("network").value);
}

$("chain-fetch").addEventListener("click", async () => {
  const status = $("chain-status");
  const target = $("chain-input").value.trim() || $("input").value.trim();
  if (!target) { status.textContent = "Read an address first, or give a box id."; status.hidden = false; return; }
  status.textContent = "Fetching…"; status.hidden = false;
  $("chain-boxes").hidden = true;
  try {
    const res = await fetch("/api/v1/lookup", {
      method: "POST", headers: { "content-type": "application/json" },
      body: JSON.stringify({ input: target }),
    });
    const body = await res.json();
    if (!res.ok) { status.textContent = `Lookup failed: ${(body.error && body.error.message) || res.status}`; return; }
    fetchedBoxes = body.boxes || [];
    if (body.height) $("height").value = String(body.height);
    if (!fetchedBoxes.length) { status.textContent = "No unspent boxes at that address."; return; }
    const sel = $("chain-boxes");
    sel.textContent = "";
    fetchedBoxes.forEach((b, i) => {
      const o = document.createElement("option");
      o.value = String(i);
      const regs = Object.keys(b.registers || {}).join(",") || "no registers";
      o.textContent = `${(b.boxId || "").slice(0, 12)}… · ${b.value} nanoERG · ${(b.tokens || []).length} token(s) · ${regs}`;
      sel.appendChild(o);
    });
    sel.hidden = false;
    status.textContent = `${fetchedBoxes.length} box(es); using the first as SELF at height ${body.height || "?"}.`;
    useFetchedBox(0);
  } catch (e) {
    status.textContent = `Lookup failed: ${e}`;
  }
});
$("chain-boxes").addEventListener("change", (e) => useFetchedBox(Number(e.target.value)));
loadConfig().then(loadRecipes);

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
