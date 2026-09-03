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
  const status = $("export-status");
  try { await navigator.clipboard.writeText(text); status.textContent = okMessage; }
  catch (e) { status.textContent = text; }
  status.hidden = false;
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

if (location.hash.startsWith("#s=")) loadShared(location.hash.slice(3));

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
loadConfig();

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
