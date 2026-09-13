# Record date convention

New records use UTC: date-only stamps explicitly say `YYYY-MM-DD UTC`; exact
instants use ISO 8601 with `Z`. A filename date for a new record uses its UTC
creation date. Do not infer an exact event time from a date-only stamp.

Historical date-only records and filenames in this repository, including
`docs/superpowers/specs/` and the discovery decision, stop, implementation and
verification records, used the author's local calendar in Australia/Brisbane
(UTC+10). Read those dates as local dates, not UTC dates. Preserve their original
text, filenames, links and evidence hashes; this convention supplies the missing
timezone context without rewriting historical substance.

In particular, the discovery records labelled **2026-09-11** were authored on
**2026-09-11 local (UTC+10), while the UTC date was 2026-09-10**, as clarified by
the author during PR #100 review. This applies to the D00 scheduling decision
and stopped/reopened attempts, D01 and D02 implementation records, D03 stopped
attempt, and corresponding design amendments and archived snapshots. It is a
timezone clarification, not a claim that events occurred in the future. No exact
event timestamps are reconstructed. Earlier local dates likewise retain their
local meaning; do not mechanically subtract one day without an event time.

This annotation changes no completion status, measurement or authorization.
Capability 1 remains the endpoint. D03 remains stopped, D04 is unregistered and
unattempted, and capabilities 2 and 3 are not planned. Synthetic agreement does
not establish utility.
