# Local-export census — 2026-09-10 UTC

A supplied local-mainnet-index export contains **350,902 distinct scripts covering
23,333,798 boxes**. The measured run lifted all 350,902; **350,898 have no raw
placeholders**, 0 are truncated, and 0 have reported parse/lift errors. This is
structural lift coverage over that export. It does not establish faithful
semantics, recompilation, contract correctness, property expressiveness or
complete historical/mainnet coverage. Four trees retain raw placeholders despite
the percentage rounding to 100.00%.

This is a **separate claim** from historical **270/279 byte-exact mainnet-tree
recompilation**. It neither supersedes that figure nor alters the current frozen
scoreboard, its denominator, or any corpus row. No recompilation was performed
by this census.

## Provenance and reproducibility

[Provenance manifest](provenance.json) records exact export and source digests,
run commands, revision, available output and missing fields. The supplied export
is `/tmp/sweep/all_trees.txt` (not copied into the repository), SHA-256:
`84acc14d88d0612787f78df7bd71fc16450bf48b8911cec5f54284b12e139472`.
The actual rerun is [rerun-all.txt](rerun-all.txt), with [progress](rerun-all.progress).
The original [FULLSCAN2.txt](FULLSCAN2.txt) and [first diagnostic](FULLSCAN.txt)
are archived unchanged; the first diagnostic printed zero text proxies and is
not silently replaced by the second run. Original source snapshots are retained
as `.rs.original` files and are not compiled examples.

**Snapshot height and snapshot date, original SQL query, index exclusions, and
original run's forge revision were not recorded in the recovered artifacts and
have not been supplied.** These fields remain null, not inferred from file mtimes,
our current HEAD, or the export counts. No index query was reconstructed and
misrepresented as the original. A clarification was requested. Therefore this is
a reproduced **supplied-export** measurement with incomplete acquisition
provenance, not an authenticated “all mainnet at height H” claim. Supplying that
metadata is the remaining provenance prerequisite; a current index tip alone
would not authenticate an older export. The rerun date and forge revision are
recorded separately and are not substitutes for snapshot date/revision.

Reproduce with the exact digest-matching local export, from repository root:

```sh
CARGO_TARGET_DIR=./target-p00 cargo build -p ergo-sandbox --release --example mainnet_scan
target-p00/release/examples/mainnet_scan < /tmp/sweep/all_trees.txt
target-p00/release/examples/mainnet_scan < /tmp/sweep/top2000.txt
```

Input format is four pipe-separated fields `hash|ergoTree|boxCount|value`, no
header. The revised scanner has no query/exclusion step: it processes every row,
rejects malformed counts/records, duplicate export hashes or decoded script
bytes, propagates stdin errors, and hashes exact input bytes including newlines.
It does not authenticate the export's hash algorithm or history. Malformed tree
hex/parsing is counted as a parse error, not silently counted as a successful
lift. Zero denominator percentages are diagnostic zero, not coverage evidence.
The current full rerun returned 0, as did the top-2,000 run.

## Findings remain unadjudicated

**432,994 findings; 109,768/350,902 scripts (31.3%) clean** under these lints.
“Clean” means no lint fired, not safe or independently reviewed. A script can
produce multiple findings of the same class. No finding was adjudicated as a
contract defect or a new vulnerability.

| Finding class | Full-export findings | Share of 432,994 findings | Top-2,000 findings | Share of 20,429 findings |
|---|---:|---:|---:|---:|
| unbound-box-reserves | 232,827 | 53.8% | 1,345 | 6.6% (about 7%) |
| unchecked-get | 198,387 | 45.8% | 18,409 | 90.1% (about 90%) |

This reproduces the **population/usage inversion**, with an additional correction:
53.8% and 45.8% are shares of **findings across the script population**, not
percentages of scripts containing that lint. The top-2,000 cohort is ranked by box
count; its finding percentages are not weighted by box count. Calling 53.8% a
script-population prevalence would be another denominator error.

[Top-2,000 output](rerun-top2000.txt) covers 22,962,174 boxes. Its export digest is
`4a82a393448db52260201aa7979a7c5d80c65f0006fb005ec8082f09e3374c72`.
All 2,000 rows match the full export byte fields and counts. The last included
box count and greatest excluded count are both 6: membership is a valid top-2,000
selection with ties, but the original SQL tie-break is unknown. The exported
membership is preserved; no re-ranking silently replaces it.

## Record the erroneous inference

The author reported a **73.6% single-cause expressiveness gap**. That inference is
unsupported. `print(lifted.node).contains(" * ")` measures **printed multiplication
prevalence**, not missing multiplication expressiveness and not the population
that arithmetic operations would unlock. Constant `scale` already exists; a
printed multiplication need not occur in the intended guarantee. Printer
formatting, constants, unreachable branches and other blockers matter.

The exact raw counts further correct the headline: printed multiplication alone
is **243,564/350,902 (69.4%)**. The literal `dataInputs` proxy is
**16,974/350,902 (4.8%)**. Their union is **258,338/350,902 (73.6%)**, representing
10,308,464 boxes (44.2% of exported boxes). **73.6% is combined printed-text
prevalence only, not a single-cause expressiveness gap.** Data-input roles and
typed integer-register reads already exist in `author-property:v1`.

The retained scanner labels these as printed-text prevalence. The original
probe also confused syntax with semantics, used a zero provenance digest,
mislabelled the contract, and compared a SC-denominated scaled amount directly
to nanoERG. Its replacement only records parser results against hashed source
material; the [separate frozen assessment](../arithmetic-assessment/REPORT.md)
reviews meaning and blockers and fails its entry threshold.
