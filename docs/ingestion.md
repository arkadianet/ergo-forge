# Static source ingestion

`ergo_sandbox::ingest::ingest_source` compiles real contract source with named
environment constants, parses the resulting wire tree and lifts it for static
tooling. `ingest_directory` recursively does the same for `.ergo` and `.es` files.
These functions do not evaluate contracts or run analysis.

```rust
use ergo_sandbox::ingest::{ingest_source, IngestOptions, Override, Status};

let mut options = IngestOptions::default();
options.overrides.insert(
    "asset".into(),
    Override::Type { r#type: "Coll[Byte]".into() },
);
let result = ingest_source("sigmaProp(SELF.tokens(0)._1 == asset)", &options);
assert_eq!(result.report.status, Status::Compiled);
let artifact = result.artifact.unwrap();
let analysis = artifact.analyze().unwrap(); // result retains the evidence case
let saved = serde_json::to_string(artifact.evidence_case().unwrap()).unwrap();
// The exported case retains each synthetic binding; deployment identity is unknown.
```

The ordinary compilation APIs retain their existing behavior. Ingestion is
opt-in and does not emit deployment addresses. Synthetic values can affect
compiler folding and remove branches; an ingested tree is not a reproduction
of the deployment tree. Use real values when exact compilation matters.

P01 attaches a versioned `evidence_case` to every report and retains it inside
successful artifacts. Use `artifact.analyze()` for provenance-bearing static
analysis; direct `tree_bytes` / `lifted` access remains a legacy, unbound API.
The new case APIs and the limits of archived records are described in
[evidence-cases.md](evidence-cases.md). No ingestion or inference rule changed.

## CLI and reports

```sh
CARGO_TARGET_DIR=./target-ing cargo build --release -p ergo-sandbox --bin ergo-es
./target-ing/release/ergo-es ingest path/to/contracts
./target-ing/release/ergo-es ingest path/to/contracts --json > ingest.json
./target-ing/release/ergo-es ingest path/to/contracts --no-infer --json > baseline.json
./target-ing/release/ergo-es ingest path/to/contracts --params overrides.json --tree-version 3
```

`--params` accepts the existing value shape plus a type-only shape:

```json
{
  "asset": {"type": "Coll[Byte]", "value": "0123456789abcdef"},
  "owner": {"type": "SigmaProp"},
  "delay": {"type": "Int", "value": 720}
}
```

Overrides are supplied first and never replaced by inference, including when
they cause a compile error. Their types also inform constraints for other free
constants. Directory overrides apply to every file; call `ingest_source` with
separate options when contracts use the same name with different meanings.
Only supported `TypedValue`/`ScriptEnv` value shapes can be compiled. Invalid
values fail explicitly rather than falling back to a synthetic value.

Every discovered file gets a deterministic relative-path row: `compiled` or
`not_compiled`, the full failure reason, bindings (type, exact value, origin and
free-use source lines), requested/actual header version, raw lifted-node count
and lift truncation. A compile error includes its source line when the compiler
provides an offset. Compilation stops at the first blocking diagnostic for each
file; a reason does not claim there are no further errors. The CLI exits nonzero
after printing the report if any file failed, or if no source files were found.
Read/UTF-8 errors are failure rows. A traversal error or symlink aborts the batch
with its path instead of silently reducing the denominator. Symlinks are not
followed. A successful compile can still have a partial lift; the report makes
that visible separately.

## Inference rules

The parser and final compiler are the pinned `ergo-compiler`. In between, a
partial constraint graph follows the parsed AST and lexical scopes. Unknown
names are supplied only when the compiler reports them missing. There is no
capitalization, prefix or suffix heuristic; `asset`, `$asset`, `CONST_ASSET` and
other identifier spellings use the same mechanism. Comments and string contents
do not participate in AST inference. Existing explicit string substitutions
remain available through value overrides; ingestion does not invent their text.

| Source use | Inference |
|---|---|
| `box.tokens(0)._1 == asset` | `asset: Coll[Byte]`, including tuple aliases and collection-property indexing |
| `box.tokens(0)._2 >= quantity`, `box.value / divisor` | `Long`, propagated across arithmetic and comparisons |
| `HEIGHT + delay`, collection indices | `Int` context |
| `sigmaProp(condition) || owner`, `owner && sigmaProp(condition)` | `SigmaProp`; Boolean operands infer Boolean when that is the known peer |
| `sigmaProp(enabled)`, `!disabled`, `if (enabled)` | `Boolean` |
| `values(i)` with a known result context | `Coll[result type]`; the index has Int context |
| `values.size`, `values.slice(...)`, `values.append(...)` on a free receiver | Collection shape; element type must come from another use |
| `val x: Long = limit`, typed lambda arguments/results | Declared types constrain the value |
| `val x = limit`, tuples, conditionals, function calls | Constraints propagate through aliases, components, branches and arguments |
| Registers, hashes, collection operations and other known methods/predefs | Signatures taken from the compiler's versioned method tables and predef environment, with fresh generic variables per use |

Constraints defer member resolution until the receiver's type is known. A
finite worklist resolves them to a fixed point. Numeric constraints select the
widest observed context (Byte through UnsignedBigInt); this is a representative
compilable width, not proof of the original Scala value's width. The final
compiler remains authoritative on overloads, promotions and valid source.

Unknown element types, unconstrained constants (`x == x`), incompatible uses
and unsupported environment types fail with the constant's name. There is no
retry search across candidate types. For example, `xs.size > 0` needs an override;
`xs(0) > SELF.value` infers `Coll[Long]`. `Coll[Int]` can be inferred but the
pinned `ScriptEnv` cannot carry it, so it is reported as unsupported. A free
name that also occurs as a local `val` in a different scope is discovered, but
the upstream binder can reject injecting it because it considers the name
already defined; that binder diagnostic is retained.

## Assumptions and limits

Synthetic values are deterministic and recorded in full: Boolean false;
Byte/Short/Int/Long values in 1..100 derived from the identifier's SHA-256;
BigInt 1; 32-byte SHA-256 byte collections; `Coll[Long]` `[1,2,3,4]`; and valid
compressed points derived deterministically from the identifier for
GroupElement and a single ProveDlog SigmaProp. Names seed distinct values only;
they never determine a type. Collection lengths, numeric values and the
SigmaProp's internal structure are assumptions, and no claim of runtime
validity or preservation of value-dependent branches is made. Unsupported
synthetic types require additional compiler environment support, not a silent
fallback. A type override controls the type, not those synthetic values.

For the two measured UnsignedBigInt failures, the public compiler accepts a
language/tree version argument but serializes using a fixed v0 header. Its
`Serializer` error occurs before the existing wrapper can stamp a v3 header.
Both public `compile` entry points have that behavior at the pinned revision;
there is no public compile option to select the serialization header. Ingestion
reports the exact construct and limitation rather than bypassing the compiler's
assembly/folding pipeline. Requested and actual versions are separate fields.

See [the Lithos measurement](ingestion-lithos-report.md) for the external
acceptance corpus, all 28 results and the remaining failures. The small fixtures
in `ergo-sandbox/tests/fixtures/ingest` test individual rules; they are not the
acceptance corpus.
