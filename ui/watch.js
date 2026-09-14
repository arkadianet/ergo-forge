"use strict";
// Source observations only. Baselines live in page memory; all data renders as text.
const Watch = (() => {
  // Mirrors ergo_sandbox::watch::MAX_REPORTS so limits fail locally with a reason.
  const MAX_PAIRS = 64;
  const pending = new WeakMap();
  const unavailable = "no explorer configured (explorer-dependency)";
  function element(root, tag, text) {
    const el = root.ownerDocument.createElement(tag);
    if (text != null) el.textContent = text;
    root.appendChild(el);
    return el;
  }
  function invalidate(root, message = "Press Watch to observe the configured source. Editing inputs resets the register baseline.") {
    pending.get(root)?.abort();
    pending.delete(root);
    root.replaceChildren();
    element(root, "p", message);
  }
  function render(reports, root) {
    if (!Array.isArray(reports) || !reports.length || reports.some(r =>
      !["bytes_match", "bytes_differ", "unverified"].includes(r.live?.status) ||
      !r.live.reason || !r.limitation || !r.observation || !r.chainSource?.kind ||
      !/^[0-9a-f]{64}$/i.test(r.nft) || !/^[0-9a-f]{64}$/i.test(r.lockfileFingerprint) ||
      (r.live.status !== "unverified" && (!Number.isInteger(r.height) || !/^[0-9a-f]{64}$/i.test(r.live.boxId))) ||
      !Array.isArray(r.registers) || r.registers.some(v => !["unchanged", "changed", "unverified"].includes(v.status) || !v.reason)
    )) throw new Error("Incomplete watch response");
    root.replaceChildren();
    for (const report of reports) {
      const row = element(root, "section");
      row.dataset.nft = report.nft;
      row.dataset.status = report.live.status;
      element(row, "h3", `NFT ${report.nft}`);
      element(row, "strong", report.live.status);
      element(row, "p", report.live.reason);
      element(row, "p", `Source: ${report.chainSource.kind} · ${report.chainSource.url ?? "no URL"} · height read: ${report.height ?? "unavailable"}`);
      element(row, "p", `Holder box: ${report.live.boxId ?? "unverified"}`);
      element(row, "p", `Lock fingerprint: ${report.lockfileFingerprint}`);
      element(row, "p", `Register baseline: ${report.baselineOrigin}`);
      if (report.registers.length) {
        const table = element(row, "table");
        const head = element(element(table, "thead"), "tr");
        for (const title of ["Register", "Status", "Current hex", "Expected hex", "Reason"]) element(head, "th", title);
        const body = element(table, "tbody");
        for (const register of report.registers) {
          const tr = element(body, "tr"); tr.dataset.status = register.status;
          for (const value of [register.name, register.status, register.current ?? "unavailable", register.expected ?? "unavailable", register.reason]) element(tr, "td", value);
        }
      } else element(row, "p", "No registers selected.");
      element(row, "p", report.limitation);
      element(row, "p", report.observation);
    }
    return reports;
  }
  async function load(payload, root, options = {}) {
    invalidate(root, "Observing the configured source…");
    const controller = new AbortController();
    pending.set(root, controller);
    const current = () => pending.get(root) === controller && !controller.signal.aborted && (options.isCurrent?.() ?? true);
    try {
      const response = await (options.request || fetch)("/api/v1/watch", {
        method: "POST", headers: { "Content-Type": "application/json" },
        body: JSON.stringify(payload), signal: controller.signal,
      });
      const reports = await response.json();
      if (!current()) return;
      if (!response.ok) throw new Error(reports.error?.message || "Watch request failed");
      const expected = payload.watches.flatMap(w => w.nfts.map(n => n.toLowerCase())).sort();
      const received = Array.isArray(reports) ? reports.map(r => r.nft).sort() : [];
      if (JSON.stringify(expected) !== JSON.stringify(received)) throw new Error("Incomplete watch response");
      return render(reports, root);
    } catch (error) {
      if (current()) invalidate(root, `Watch unavailable: ${error.message}. No observation was inferred.`);
    }
  }
  function mount(root, options = {}) {
    const get = id => root.querySelector(`#watch-${id}`);
    const result = get("result");
    let cfg = { explorer: false };
    let generation = 0;
    let busy = false;
    let baselines = {};
    const enabled = () => { get("run").disabled = !cfg.explorer || busy; };
    function reset() {
      ++generation;
      baselines = {};
      busy = false;
      invalidate(result);
      enabled();
    }
    for (const id of ["lockfile", "nfts", "registers", "baseline"]) get(id).addEventListener("input", reset);
    get("reset").onclick = reset;
    function configure(value) {
      reset();
      cfg = value;
      get("availability").textContent = cfg.explorer
        ? `Uses this instance’s configured ${cfg.network || ""} explorer when you press Watch. Observations are of its response.`
        : unavailable;
      enabled();
    }
    get("file").onchange = async () => {
      reset();
      const version = generation;
      const file = get("file").files?.[0];
      if (!file) return;
      get("lockfile").value = "";
      busy = true; enabled();
      try {
        if (file.size > 1024 * 1024) throw new Error("Lockfile exceeds 1 MiB");
        const text = await file.text();
        if (version !== generation) return;
        JSON.parse(text);
        get("lockfile").value = text;
        invalidate(result, "Lockfile loaded. Press Watch to observe its NFT holders.");
      } catch (error) {
        if (version === generation) invalidate(result, `Lockfile unavailable: ${error.message}`);
      } finally {
        if (version === generation) { busy = false; enabled(); }
      }
    };
    async function run() {
      if (!cfg.explorer) { invalidate(result, unavailable); return; }
      if (busy) return;
      const version = ++generation;
      busy = true; enabled();
      try {
        const lockfile = JSON.parse(get("lockfile").value);
        const split = text => text.trim().split(/[\s,]+/).filter(Boolean);
        const nfts = split(get("nfts").value).map(n => n.toLowerCase());
        const registers = split(get("registers").value);
        if (!nfts.length || nfts.some(n => !/^[0-9a-f]{64}$/.test(n))) throw new Error("Enter 64-hex NFT IDs");
        if (new Set(nfts).size !== nfts.length) throw new Error("NFT IDs must be unique");
        if (nfts.length > MAX_PAIRS) throw new Error(`At most ${MAX_PAIRS} NFTs per watch`);
        if (registers.some(r => !/^R[4-9]$/.test(r))) throw new Error("Register names must be R4–R9");
        if (new Set(registers).size !== registers.length) throw new Error("Register names must be unique");
        const supplied = get("baseline").value.trim();
        const baselineBoxes = supplied ? Object.fromEntries(nfts.map(n => [n, JSON.parse(supplied)])) : { ...baselines };
        const reports = await load({ watches: [{ lockfile, nfts, registers, baselineBoxes }] }, result,
          { request: options.request, isCurrent: () => generation === version });
        if (generation !== version || !reports) return;
        for (const report of reports) {
          if (report.baselineBox && !baselines[report.nft]) baselines[report.nft] = report.baselineBox;
        }
        return reports;
      } catch (error) {
        if (generation === version) invalidate(result, `Watch input error: ${error.message}`);
      } finally {
        if (generation === version) { busy = false; enabled(); }
      }
    }
    get("run").onclick = run;
    configure(cfg);
    return { configure, run, reset };
  }
  return { render, load, invalidate, mount };
})();
if (typeof module !== "undefined") module.exports = Watch;
