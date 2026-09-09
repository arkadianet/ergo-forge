# Corpus audit sweep

Runs all five audit lints across every `.es` file in a corpus directory and
emits one JSON record per contract. Its purpose is to measure **the detectors**,
not to publish judgements about the protocols in the corpus.

```sh
cargo run --release -p ergo-sandbox --example audit_sweep > sweep.json
```

An optional positional argument selects another corpus directory. Discovery is
recursive and sorted, with no allowlist. Directory enumeration failure is fatal;
per-contract read, parameter and compilation errors are retained as records so
an unanalysable contract is visible rather than silently absent. An empty corpus
is an error. JSON goes to stdout, diagnostics to stderr.

It is an **example binary** rather than a subcommand so that scanning a corpus
and producing verbose output stays an explicit act.

## What a sweep does and does not establish

A finding is a **syntactic observation about one tree**. It is not a defect, not
a deployment, and not an exploit. Counts are occurrences: duplicated corpus
variants count separately, and one underlying pattern can raise many. Nothing in
a sweep evaluates a transaction — for whether value can actually leave a
contract, that is the drain hunt's job, and its answer comes from the consensus
reducer.

Reporting a rate honestly means saying who the population is. Our corpus is
mostly *other people's deployed contracts*, which is what makes it a fair test
of the detectors and also why per-protocol results are handled under
[disclosure](#disclosure) rather than committed here.

## Baseline (merged main `e1e7218`, 108 files)

| | |
| --- | --- |
| compiled and lifted completely | 96 |
| compiled, lifted partially | 2 |
| could not compile | 10 |

| Lint | Findings | Files | HIGH | MED | LOW |
| --- | --- | --- | --- | --- | --- |
| unchecked-get | 424 | 51 | 302 | 29 | 93 |
| unbound-box-reserves | 74 | 33 | 74 | 0 | 0 |
| delegated-reserves | 14 | 12 | 0 | 14 | 0 |
| height-guards | 6 | 6 | 0 | 1 | 5 |
| trust-assumptions | 4 | 3 | 0 | 4 | 0 |

522 findings across 74 files; 24 analysed files were clean.

## Detector limitations this sweep exposed

The sweep's most useful output is the list of places our own lints are wrong.
No detector was changed while measuring, so these are open:

- **`unchecked-get` does not canonicalise box aliases.** It compares rendered
  receiver expressions and follows Boolean guard aliases, so a contract that
  proves `OUTPUTS(0).R4[Long].isDefined` in a guard and then reads the register
  through an alias (`successor = OUTPUTS(0)`) is reported HIGH even though the
  read is guarded. Confirmed by hand on a corpus contract. This is a substantial
  share of the 302 HIGHs.
- **`unbound-box-reserves` HIGH wording is too broad for payment outputs.** An
  ordinary recipient output pinned to specific proposition bytes and a required
  value does not need a singleton NFT, but is flagged as though it does.
- **`delegated-reserves` cannot see issuance supply,** so a singleton NFT whose
  amount is guaranteed by supply-1 issuance still reads as an unbounded token
  slot. Correct given what the lint can observe; noise to a reader.
- **The lints are whole-tree and branch-insensitive.** A known-patched contract
  and its pre-patch original produce identical counts here. Silence therefore
  does **not** validate a patch, and this sweep must never be cited as though it
  did.

Two contracts lift only partially and four findings come from those partial
trees; a partial lift can both miss and misattribute.

## Disclosure

The corpus contains real, deployed third-party protocols. Findings that name a
protocol, file or line go to that protocol's team privately first. Per-protocol
results are published here only after a fix, or on a window agreed with the
team. A sweep's aggregate numbers and our own detectors' failures — the two
things above — carry no such constraint and are what this document records.
