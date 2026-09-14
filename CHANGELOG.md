# Changelog

## 0.5.0 — prepared, unreleased

This release collects roadmap v2 batches 0–10. Cutting `v0.5.0` after merge is
the maintainer's action; this preparation does not publish binaries or a container.

| Batch | Units | What a user gets |
|---|---|---|
| 0 | W00, S00 | The playground roadmap and an attack-vector catalogue with examples and explicit manual checks. |
| 1 | W01 | Attacker controls in Play and the guided USE drain walkthrough, with synthetic results. |
| 2 | S02 | Four new static lints, mutant/control pairs and a recorded deployed-contract sweep. |
| 3 | S01, W06 | A provenance-labelled checklist in Write and Read, plus anchored observations of what a contract does not check. |
| 4 | I01, I02 | Exact/template deployment comparison and contract lockfiles for project exports and CI drift checks. |
| 5 | W02, W03 | Export Play drafts as CLI tests/scenarios and share capped Play chains and suites offline. |
| 6 | W04 | Cost per source span and expression explanations in Write. |
| 7 | S04, I04 | Immobilisation probes, a static storage-rent explanation and upgrade-hook review observations. |
| 8 | I03 | Watch protocol NFTs for script and upgrade-hook changes, with fixture-backed checks. |
| 9 | W05, S05 | Pool-bound swap and successor-locked vault recipes with caught mutants; incident scaffolds with expectations left to the author. |
| 10 | X01, X02, X03 | Engine remeasurement, release 0.5.0 preparation, a cached node corpus checkout and parallel cost-trace CI. |

Engine pin: `9468043396e5daa2828211bcff4234bc70fae4f0` →
`016533194f94ad95b1a87df70bb9bfce493922e2`. Seed exact round-trips rose
from 74/87 to 79/92; mainnet stayed 270/279. All nine P03 vectors retained
their outcomes; no previously exact seed or mainnet row was lost. The compiler, reducer
and validator continue to share one Cargo pin. See `docs/reports/batch-10/` for
the measured revision pair and exact per-corpus outcomes.

The existing test action downloads the matching `ergo-es` asset from a named
tag or the latest release. The 0.5.0 source includes `verify-lock`, satisfying
its lockfile compatibility probe once `v0.5.0` becomes the latest release.

## 0.4.1 — 2026-09-07

Tag `v0.4.1` points to `2a3726251c80ed40a57963681ade15f5bcb59769`:
“feat(cli): --json for eval and test; docs/scenario-format.md, the stable v1
contract for a second reducer” (#58). This is the only commit after `v0.4.0`.
The tagged workspace still declared crate version 0.3.0.

## 0.4.0 — 2026-09-07

Tag `v0.4.0` points to `ce15a9ba57763f60838b49c67f6e1a14f72617af`:
“feat(sandbox): every input carries its own context extension” (#56).
The history since `v0.3.0` also added Play, plain-word contract descriptions,
walkthroughs, source values after evaluation and the method sweep, with UI
and tree-version fixes (#47–#55). The tagged workspace still declared 0.3.0.

## 0.3.0 — 2026-09-05

Tag `v0.3.0` points to `4e3af4de9db0111deb105ee76ba6ac185c8c9deb`,
“chore(release): 0.3.0” (#46). Its annotated tag describes the rule composer,
16 recipes, independently tested protocol examples, spending and multiparty
proofs, AVL+ support and a node pin at `8e2eb13`. The release commit updated
the workspace version, lockfile, action example and README.
