# P3a — static audit lints — design

Date: 2026-09-01
Status: proposed
Depends on: P2.5 (public lifted AST) — PR #4
Scope: the lint framework and one lint end-to-end. Not the whole audit layer.

## Why this is scoped to one lint

P3 in `workbench-PLAN.md` bundles three independent subsystems: static lints over
the lifted tree, scenario fuzzing over the evaluator, and cost hot-spot views off
`cost-trace`. Only the first depends on P2.5; the other two don't touch the AST.
They get their own specs.

Within static lints, this spec builds the framework plus **one** lint —
unchecked `Option.get` — as a vertical slice. The framework's shape is the risky
part; writing lint number two through ten is repetition once it's proven. Picking
a second lint before the first one has run against real contracts is guessing.

## The product question this answers

An auditor pastes a mainnet address and gets back findings about the contract
that will actually execute. That is the capability nothing else in the ecosystem
has, and it is why the audit layer runs on the lifted tree rather than on
authored source (see `2026-08-31-lift-target-ast-design.md`).

## Non-negotiable: a lint that cries wolf is worse than no lint

Mainnet contracts hold real money. A lint firing on most of the corpus is noise,
and noise trains people to ignore the tool. This spec therefore makes the
false-positive rate a **measured acceptance criterion**, not an aspiration — see
Testing.

## Architecture

```
Lifted { node, raw_placeholders, truncated }        (from P2.5)
      │
      ▼
audit::audit(&Lifted) ──> Audit { findings, completeness }
      │                          │
      │                          └── incomplete when the lift left Raw
      │                              placeholders or truncated: the audit
      │                              did not see the whole contract and
      │                              must say so
      ▼
  each lint fn: &Node -> Vec<Finding>
```

### Module layout

New module, `ergo-sandbox/src/audit/`:

| File | Responsibility |
|---|---|
| `mod.rs` | `audit()`, `Audit`, `Completeness`, the `LINTS` registry |
| `finding.rs` | `Finding`, `Severity` |
| `visit.rs` | `visit(&Node, &mut impl FnMut(&Node))` — shared preorder walk |
| `lints/mod.rs` | re-exports the lint fns |
| `lints/unchecked_get.rs` | the first lint |

One lint per file. The registry is a plain array — no trait objects, no dynamic
registration, until something needs them.

### Types

```rust
/// How much a finding should alarm a reader.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    /// Can cause the script to fail at validation, locking the box.
    High,
    /// Suspicious or fragile; may be intentional.
    Medium,
    /// Informational.
    Low,
}

/// Longest rendered snippet carried on a finding; longer ones are cut with
/// a trailing `…`. Keeps a finding printable on one terminal line.
pub const SNIPPET_MAX: usize = 120;

/// One lint result, anchored to a node in the lifted tree.
#[derive(Debug, Clone)]
pub struct Finding {
    /// Stable lint identifier, e.g. `"unchecked-get"`. Machine-readable.
    pub lint: &'static str,
    pub severity: Severity,
    /// `Node::id` of the offending node. Lift-local — see `ast::Node::id`.
    /// This is what a future source map turns into a line number.
    pub node_id: u64,
    /// One sentence, specific to this occurrence.
    pub message: String,
    /// The offending subtree rendered back to source, so the finding is
    /// readable with no source map and no original source. Truncated to
    /// `SNIPPET_MAX` chars.
    pub snippet: String,
}

/// Whether the audit saw the whole contract.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Completeness {
    /// Every construct lifted; findings cover the whole tree.
    Complete,
    /// The lift left raw placeholders or hit the depth ceiling. Some of the
    /// contract was not analysed — absence of findings proves nothing.
    Partial { raw_placeholders: usize, truncated: bool },
}

#[derive(Debug, Clone)]
pub struct Audit {
    pub findings: Vec<Finding>,
    pub completeness: Completeness,
}
```

`Completeness` is not decoration. It carries the same discipline the decompiler
already applies to its output: a tree containing `NodeKind::Raw` was not fully
understood, so an audit over it must not read as a clean bill of health. Any
shell rendering an `Audit` must display `Partial` prominently.

### The registry and entry point

```rust
/// Every lint, in the order findings are reported.
const LINTS: &[fn(&Node) -> Vec<Finding>] = &[lints::unchecked_get];

pub fn audit(lifted: &Lifted) -> Audit {
    let mut findings: Vec<Finding> =
        LINTS.iter().flat_map(|lint| lint(&lifted.node)).collect();
    findings.sort_by_key(|f| (f.severity as u8, f.node_id));
    Audit {
        findings,
        completeness: if lifted.raw_placeholders == 0 && !lifted.truncated {
            Completeness::Complete
        } else {
            Completeness::Partial {
                raw_placeholders: lifted.raw_placeholders,
                truncated: lifted.truncated,
            }
        },
    }
}
```

Findings sort by severity then node id, so output is deterministic — required for
testable CLI output and for diffing two audits of the same contract.

## The lint: unchecked `Option.get`

### What it detects

`Option.get` on a register or context read with no `isDefined` guard. If the
option is empty at validation time the script throws, the spend fails, and — for
a contract whose only spending path goes through that read — the box is
permanently unspendable.

### Shapes, verified against the current lift

| Source | Lifted shape |
|---|---|
| `SELF.R4[Int].get` | `Method(recv, "get", [])` |
| `SELF.R4[Int].isDefined` | `Method(recv, "isDefined", [])` |
| `x.getOrElse(d)` | `Method(recv, "getOrElse", [d])` |
| `coll.get(i)` | `Method(recv, "get", [i])` — **one arg** |

The discriminator is arity: `Option::get` takes no arguments;
`SCollection::get` (method table `0x0C/0x21`) and `AvlTree::get` (`0x64/0x0A`)
both take one. **The lint matches only `Method(_, "get", args)` where
`args.is_empty()`.** `getOrElse` is the safe form and is never flagged.

### Guard analysis

A `get` is **guarded** when an `isDefined` on the same receiver dominates it.
Two forms are recognised, both confirmed against real compiler output:

1. **Conjunction** — `x.isDefined && x.get > 5`.
   Lifts to `Infix("&&", lhs, rhs)` where `lhs` contains
   `Method(r, "isDefined", [])` and `rhs` contains `Method(r, "get", [])`.
   The compiler CSEs the receiver, so `r` is typically `Val("%1")` on both
   sides — syntactic equality is sufficient and is what the lint uses.
2. **Conditional** — `if (x.isDefined) x.get else d`.
   Lifts to `If(cond, then_branch, _)` with the `isDefined` in `cond` and the
   `get` in `then_branch`.

Implementation: a recursive walk carrying a set of receivers known non-empty in
the current scope. Entering the rhs of `&&`, add every receiver that the lhs
proves defined; entering the then-branch of an `If`, add every receiver its
condition proves defined. A `get` whose receiver is in the set is not reported.

Receiver identity is **syntactic equality of the rendered receiver** — compare
`print(recv)`. This is deliberately conservative and its limits are stated below.

### What it deliberately does not catch

Stated here so the gaps are documented rather than discovered:

- **`||` and negation.** `!x.isDefined || x.get > 5` is safe but will be
  reported. Rarer than the `&&` form; adding it means real boolean reasoning.
- **Guards through a `val` in an enclosing block.** `val ok = x.isDefined`
  followed by `ok && x.get` — the guard is behind a binding the lint does not
  follow.
- **Cross-branch reasoning**, e.g. an earlier `if` that already returned.
- **Semantic receiver equality.** Two spellings of the same box read compare
  unequal; two syntactically identical reads of *different* boxes would compare
  equal (not possible today, since receivers carry their index).

The first two produce **false positives**, which is why the corpus measurement
below is a gate rather than a report.

### Severity

`High`. An unguarded `get` that fails makes the box unspendable through that
path — the highest-consequence class this lint can produce.

## CLI surface

```
ergo-es audit <tree-hex>            # audit one tree
ergo-es audit --seed | --mainnet    # audit a corpus, summary tallies
```

Output for a single tree:

```
audit: 2 finding(s)  [complete]

HIGH  unchecked-get  node 47
  Option.get with no isDefined guard — the script throws if the register is
  empty, making this spending path unusable.
  SELF.R4[Int].get
```

`[complete]` becomes `[PARTIAL: 3 raw placeholders]` when the lift was
incomplete, and that line is the first thing printed, not a footnote.

## Error handling

`audit` is total and cannot fail: it consumes an already-lifted tree, so
malformed input was rejected earlier at `parse_tree`. No new error variants in
`SandboxError`. A lint that cannot decide reports nothing — silence means "not
flagged", and `Completeness` carries "not fully seen" separately.

## Testing

Three layers.

**1. Unit — the lint's decisions.** Source in, findings out, over compiled
trees. Must include, at minimum: bare `SELF.R4[Int].get` (flagged);
`isDefined && get` (not flagged); `if (isDefined) get else d` (not flagged);
`getOrElse` (not flagged); `coll.get(i)` — one arg (not flagged, wrong `get`);
two independent unguarded gets (two findings, distinct `node_id`s).

**2. Framework.** `Completeness::Complete` on a fully-lifted tree; `Partial`
with the right counts on a tree containing a raw placeholder; findings sorted
deterministically.

**3. Corpus measurement — the acceptance gate.** Run the lint over the 279
mainnet trees and the seed corpus, and record the numbers in
`ergo-sandbox/README.md`:

- how many trees produce at least one finding;
- total findings;
- how many audited trees were `Partial`.

Then **hand-verify a sample of at least 10 flagged trees**, decompiling each and
deciding whether the finding is real. Record the verdict per sample.

**The gate:** if more than **20%** of mainnet trees are flagged, stop and report
rather than shipping. At that rate the lint is either wrong or the pattern is so
common in real contracts that `High` is the wrong severity — either way it needs
a decision, not a merge. This number is a first estimate and may be revised once
measured, but it must be revised *deliberately*, in the PR, with the measurement
visible.

## Out of scope

Additional lints, scenario fuzzing, cost hot-spots, the source map, WASM
bindings, and any REST surface. Each is separate work; this spec exists to make
the first of them cheap.
