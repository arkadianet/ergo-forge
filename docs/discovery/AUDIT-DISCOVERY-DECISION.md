# Audit discovery decision — 2026-09-26 UTC

**Status:** user-authorized implementation decision for the stacked branch
`feature/audit-discovery`, based on `security/forge-hardening` / PR #113.

## Decision

Authorize a bounded, evidence-first discovery workbench on top of the pinned
Forge engine. The work is permitted to add coverage reporting, property
templates, seeded multi-step adversary search, flow review obligations, a
holdout cohort and a strictly weaker shadow consistency model.

This is an explicit exception to the discovery-axis freeze recorded in
`docs/discovery/CONSOLIDATION-DECISION.md`. It does not rewrite the frozen
roadmap policy blocks, does not reopen M05 or D03, and does not claim that any
previous stop was passed.

## Non-authorizations

- No second consensus reducer, acceptance oracle or property-violation
  authority.
- No use of `txcheck::check`, Play, adversary search, static analysis or the
  shadow model as node acceptance.
- No safety score, coverage percentage, "not vulnerable" result or automatic
  discovery credit.
- No publication of confidential contract sources, witnesses or premises.
- No new protocol, wallet, broadcasting or live mutation capability.

## Required evidence boundary

Every new surface carries one of the existing provenance labels and keeps
`nodeValidated: false` unless it enters the pinned full validator through the
existing evidence path. Absence under a cap remains unknown. Caps, truncation,
rejections, engine revision and replay identity are part of the result.

The holdout namespace starts with an explicit zero discovery-eligible
denominator. Shadow-model agreement is bookkeeping consistency only; it cannot
promote a candidate or close a property question.

## Stop conditions

Stop and record the failure if any of these occur:

1. a new instrument claims node acceptance or a confirmed violation without
   the pinned validator;
2. an unmodelled construct is treated as clean rather than unknown;
3. a benchmark result changes because of post-result cap or seed tuning;
4. holdout exposure is misreported or a self-consistent row earns discovery
   credit;
5. a bounded trace or property template escapes its declared resource ceiling.

## Measurement

The first measurements are capability and self-consistency measurements, not
utility claims. Utility requires an independently reviewed transfer cohort and
a governing checkpoint. The v2 roadmap's D03 stop and E01 precondition remain
in force for any later claim about novel discovery.
