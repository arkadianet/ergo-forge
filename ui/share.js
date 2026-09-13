"use strict";
// Fragment-only, offline sharing. No network, storage, compression or CDN.
const Share = (() => {
  // 7 KiB including #s= leaves room for an ordinary origin/path below 8 KiB.
  const FRAGMENT_CAP = 7 * 1024;
  const PREFIX = "#s=";
  const object = value => value !== null && typeof value === "object" && !Array.isArray(value);
  const network = value => value === "mainnet" || value === "testnet";
  const uint = value => Number.isSafeInteger(value) && value >= 0;
  const hex = value => typeof value === "string" && /^(?:[0-9a-f]{2})+$/i.test(value);
  function requireThat(ok, message) { if (!ok) throw new Error(`Invalid share: ${message}`); }
  function checkSize(size) {
    if (size > FRAGMENT_CAP) throw new Error(`Share fragment is ${size} bytes; cap is ${FRAGMENT_CAP} bytes.`);
  }
  function validateSuite(suite) {
    requireThat(object(suite), "suite must be an object");
    requireThat(suite.nodeValidated == null || suite.nodeValidated === false, "shared suites are not node-validated");
    requireThat((typeof suite.source === "string") !== (typeof suite.tree === "string"), "suite needs exactly one source or tree");
    requireThat(suite.tree == null || hex(suite.tree), "suite tree must be hex");
    requireThat(suite.network == null || network(suite.network), "unknown suite network");
    requireThat(suite.params == null || object(suite.params), "suite params must be an object");
    requireThat(Array.isArray(suite.scenarios), "suite scenarios must be an array");
    for (const c of suite.scenarios) {
      requireThat(object(c) && typeof c.name === "string" && uint(c.height) && c.height <= 0xffffffff, "invalid suite case");
      requireThat(["pass", "fail", "error", "needsProof", "proofAccepted", "proofRejected"].includes(c.expect), "unknown expectation");
      requireThat(c.source == null && c.tree == null, "only the suite may name the contract");
      requireThat(c.nodeValidated == null || c.nodeValidated === false, "shared cases are not node-validated");
    }
    return suite;
  }
  function validate(state) {
    requireThat(object(state), "state must be an object");
    if (!Object.hasOwn(state, "k")) {
      requireThat(typeof state.s === "string", "Write source must be a string");
      requireThat(state.p == null || object(state.p), "Write params must be an object");
      requireThat(state.n == null || network(state.n), "unknown Write network");
      return state;
    }
    requireThat(state.v === 1, "unsupported version");
    requireThat(state.k === "play" || state.k === "suite", "unknown kind");
    requireThat(state.nodeValidated == null || state.nodeValidated === false, "shared experiments are not node-validated");
    if (state.k === "suite") validateSuite(state.suite);
    else {
      requireThat(uint(state.height) && state.height <= 0xffffffff, "height must be a u32");
      requireThat(network(state.network), "unknown Play network");
      requireThat(Array.isArray(state.boxes), "boxes must be an array");
      const ids = new Set();
      for (const b of state.boxes) {
        requireThat(object(b) && hex(b.boxId) && b.boxId.length === 64, "boxId must be 32-byte hex");
        requireThat(!ids.has(b.boxId.toLowerCase()), "duplicate boxId"); ids.add(b.boxId.toLowerCase());
        requireThat(hex(b.ergoTree), "ergoTree must be hex");
        requireThat(uint(b.value), "box value must be an exact nonnegative integer");
        requireThat(b.creationHeight == null || (uint(b.creationHeight) && b.creationHeight <= 0xffffffff), "invalid creation height");
        requireThat(b.spent == null || typeof b.spent === "boolean", "spent must be boolean");
        requireThat(b.registers == null || object(b.registers), "registers must be an object");
        for (const tv of Object.values(b.registers || {})) requireThat(object(tv) && typeof tv.type === "string" && Object.hasOwn(tv, "value"), "registers need typed values");
        requireThat(b.tokens == null || Array.isArray(b.tokens), "tokens must be an array");
        for (const t of b.tokens || []) requireThat(object(t) && hex(t.id) && t.id.length === 64 && uint(t.amount) && t.amount > 0, "invalid token");
      }
      requireThat(state.history == null || (Array.isArray(state.history) && state.history.every(line => typeof line === "string")), "history must be an array of text entries");
    }
    return state;
  }
  function encodeShare(state) {
    validate(state);
    const bytes = new TextEncoder().encode(JSON.stringify(state));
    // Check before building a potentially large base64 string. The fragment is ASCII.
    checkSize(PREFIX.length + Math.ceil(bytes.length * 4 / 3));
    let bin = ""; for (const byte of bytes) bin += String.fromCharCode(byte);
    return btoa(bin).replace(/\+/g, "-").replace(/\//g, "_").replace(/=+$/, "");
  }
  function decodeShare(fragment) {
    requireThat(typeof fragment === "string", "fragment must be text");
    const frag = fragment.startsWith(PREFIX) ? fragment.slice(PREFIX.length) : fragment;
    checkSize(PREFIX.length + new TextEncoder().encode(frag).length);
    requireThat(/^[A-Za-z0-9_-]+$/.test(frag) && frag.length % 4 !== 1, "malformed base64url");
    const bin = atob(frag.replace(/-/g, "+").replace(/_/g, "/") + "=".repeat((4 - frag.length % 4) % 4));
    const bytes = Uint8Array.from(bin, c => c.charCodeAt(0));
    return validate(JSON.parse(new TextDecoder("utf-8", { fatal: true }).decode(bytes)));
  }
  // The caller alone owns chain replacement. Loading or cancelling never calls it.
  function confirmPlay(state, root, replace) {
    validate(state);
    requireThat(state.k === "play", "expected a Play chain");
    const snapshot = JSON.parse(JSON.stringify(state));
    root.replaceChildren(); root.hidden = false;
    const doc = root.ownerDocument;
    const note = doc.createElement("p");
    note.textContent = `Shared synthetic Play chain: ${state.boxes.length} boxes at height ${state.height} (${state.network}). Not node-validated. Replace your saved browser chain?`;
    root.appendChild(note);
    const confirm = doc.createElement("button"); confirm.type = "button"; confirm.textContent = "Replace my Play chain";
    const cancel = doc.createElement("button"); cancel.type = "button"; cancel.textContent = "Keep my chain";
    let pending = true;
    confirm.onclick = () => {
      if (!pending) return;
      replace(snapshot);
      pending = false; root.replaceChildren(); root.hidden = true;
    };
    cancel.onclick = () => { pending = false; root.replaceChildren(); root.hidden = true; };
    root.append(confirm, cancel);
  }
  return { FRAGMENT_CAP, encodeShare, decodeShare, validate, validateSuite, confirmPlay };
})();
if (typeof module !== "undefined") module.exports = Share;
