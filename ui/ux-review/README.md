The UX pass makes **Read → related contracts → test the displayed contract**
a complete workflow. All changes are in `ergo-web/` and `ui/`, on
`feat/ux-pass`. No commits or pushes were made. The engine crate and the live
service on port 8090 were left untouched.

I read the September 1 web design, September 3 playground design (including
“Why the reader is not a playground”), and the workbench audience plan before
changing the app. The assessment used native headless Chromium at desktop
1440×1100 and mobile 390×844. Findings below concern product behavior.

| Rank | Severity | Observed gap | Disposition |
| --- | --- | --- | --- |
| 1 | High | Read's Scenario/Tests used the editor's contract. Repro: compile `HEIGHT > 1000`, read the R4 register example, run at height 1001. Scenario said PASS for the height lock while the reader displayed a different contract. | Fixed. Reader suites, exports and implicit scenarios use the recovered tree. Context is labeled; changed input and late responses cannot reuse stale results. |
| 2 | High | Pasting a box ID into Read produced a low-level ErgoTree parse error. Lookup was available only below an existing result, and offline instances hid it without explaining the limitation. The old box selector only replaced SELF; it did not read the selected box's script. | Fixed. Direct box-ID reading, clear offline/network messaging, explicit input type for 32-byte trees, and a visible chain-box picker that recovers the selected box's source. Verified with two different box scripts from a local mock explorer. |
| 3 | High | No protocol map route or reading surface, despite a public engine API. Readers could not follow a contract's references to related scripts. | Added `/api/v1/map`, typed references, exact box/address/token/transaction seeds, source navigation, offline recordings, a synthetic learning example and JSON download. Unresolved references, partial nodes and traversal limits stay visible. |
| 4 | Medium | The differentiator was hard to discover: fresh browsers opened Build, returning browsers opened Write; the product was titled “playground.” Read exposed three large developer panels before a learner needed them. | Read is now the default. Entry copy explains source recovery. Testing tools are collapsed behind an explicit action, while Write keeps its editor and tabs. Explicit `#build`, `#write`, `#play` and existing shared-source links still work. |
| 5 | Medium | Reading stayed busy through automatic probes, context could linger between reads, and source recovery had no positive completeness label or copy action. | Source becomes usable as soon as inspection finishes. Probes have separate status and response ownership. Added completeness/provenance labels, Copy source, visible focus and keyboard tab navigation. |
| 6 | Medium | Drain hunt exists in the engine but has no web route or UI. | Confirmed and deliberately deferred. It needs a coherent multi-box transaction/role/setup workflow. Adding a raw JSON panel would not complete this reader-focused slice. |
| 7 | Medium | Testing requires JSON knowledge; example suites assume a particular height-lock contract. The examples menu is long, has repeated names across groups, and needs browsing/search affordances. | Deferred. Preserved the existing authoring model and editable scenarios. |
| 8 | Low | Validate reports an empty transaction with no inputs/outputs as “Would validate,” which can look like a useful successful check to a first-time user. | Deferred. This pass did not change transaction semantics or the engine. |

The map is a box-card view plus a reference table. It describes relationships
without requiring a graph layout library, canvas interaction or bundler.
Clicking a box recovers its source through the ordinary inspect endpoint.
The map response is an explicit web DTO, with nanoERG amounts represented as
strings; it does not expose the engine's Rust types as a wire contract.
The existing engine budget/large-stack execution path handles traversal.
Fixtures never contact the explorer, including when they contain a URL.

The focused slice leaves Build recipes, the composer, the editor, and Play's
transaction builder in place. Existing hunt behavior was exercised using
synthetic fixtures; no contract security review or drain-hunt work was done.

| Surface exercised | Result |
| --- | --- |
| Read / inspect / examples / hunt | Address/tree recovery and all six probe rows; malformed input and direct box-ID offline feedback; recovered-source navigation from the map. |
| Compile / Write examples | Parameterized height-lock example loads; valid source compiles; malformed source produces an error; the existing two-case height-lock suite passes. |
| Eval / tests | Write's height-lock scenario passes at 1001. Read's register contract correctly errors without R4; its named error case passes. Delayed evaluation cannot paint a result after switching modes. |
| Lookup | Default 501 behavior confirmed. A configured local mock provides box data, raw R4, height and two selectable scripts. Changing selection changes both source and SELF; wrong-network lookup is explained. |
| Build | Recipe gallery and the lock walkthrough's entry step exercised. |
| Point | The existing test-key form derives and displays the public point for scalar 1. |
| Play | Fund a synthetic box, select it, submit a balanced same-script output; transaction accepted. |
| Validate | Request submitted through the browser; existing empty-transaction acceptance recorded above. |
| Protocol map | Offline and configured-explorer traversal, all four seed kinds over HTTP, related-source reads, capped traversal/unresolved references, missing fixture evidence, invalid limits, network mismatch and missing seed. |
| Desktop / mobile / keyboard | Entry and map screenshots inspected; no page-level horizontal overflow at 390px. Map's input remains full width. Arrow/Home/End navigation works across the mode tabs. No uncaught browser errors. |

Verification completed:

- `CARGO_TARGET_DIR=/home/rkadias/.cache/cargo-target cargo test --release --workspace`
  — **301 passed, 0 failed**, including six new map HTTP tests.
- `CARGO_TARGET_DIR=/home/rkadias/.cache/cargo-target cargo clippy --release --workspace --all-targets -- -D warnings`
  — clean.
- `cargo fmt -p ergo-web`, then `cargo fmt --all --check` — clean.
- Native Chromium regression runner in `ergo-web/tests/browser.mjs` — passed;
  Node syntax check and `git diff --check` — clean.

Chain-dependent browser verification used the local synthetic explorer in
`ergo-web/tests/reader-explorer.py`. Public-explorer latency, production
availability and real-chain protocol completeness were not measured. The
pre-existing engine explorer client retries requests, so live maps can take
minutes. Cancelling a browser request prevents stale rendering; the bounded
engine job still runs to completion. The report does not treat that as
server-side cancellation.

Public engine API boundaries encountered:

- There is no `Seed::BoxId`. A small `ChainSource` adapter provides a one-box
  initial frontier through the existing address-seed API, delegates all other
  queries, and reports the honest `boxId` seed in the web DTO. No engine change
  was necessary.
- `map::source::ChainBox` does not carry registers. A mapped script can be
  opened offline, but a complete spending context cannot be reconstructed
  from that map/fixture type. The UI labels source-only navigation and clears
  previous box context. A separate configured lookup can fetch actual SELF.
- The public explorer has no script-hash index for the hash used by the
  engine. Those references remain unresolved. Dynamic references and boxes
  outside the traversal bounds also remain outside any completeness claim.
- The live map API has no progress/cancellation hook or pinned historical
  snapshot; the web shell cannot provide those guarantees without further
  source/engine support. The reported height is the traversal's reference
  height, not a promise that every upstream response came from one immutable
  snapshot.

Screenshots (captured from the actual app, with synthetic example data):

| View | Before | After |
| --- | --- | --- |
| Entry | [Fresh browser opens Build](screenshots/before-entry.png) | [Reader entry](screenshots/after-entry.png) |
| Reader | [Original read result](screenshots/before-read.png) | [Recovered source and context](screenshots/after-read.png) |
| Box ID, offline | [Low-level parse error](screenshots/before-box-id.png) | [Actionable lookup explanation](screenshots/after-box-id-offline.png) |
| Protocol map | No surface existed | [Typed references and source actions](screenshots/after-map.png), [limits and unresolved references](screenshots/after-map-limits.png) |
| Configured box lookup | — | [Box source with actual SELF](screenshots/after-live-box.png) |
| Reader testing | — | [Recovered-tree suite](screenshots/after-reader-tests.png) |
| Mobile | — | [Reader entry](screenshots/after-mobile-entry.png), [protocol map](screenshots/after-mobile-map.png) |

Additional baseline captures: [Write/Validate](screenshots/before-write.png),
[Play](screenshots/before-play.png). Reproduction commands and the map API
contract are documented in `ergo-web/README.md`.

A captured [example map download](example-map.json) is included for API review.
