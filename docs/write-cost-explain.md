# Write: cost by source and Explain

These are synthetic sandbox diagnostics (`nodeValidated: false`), not safety
claims or node validation. All three routes use the existing engine budget,
request limit and rate limiter. Each request calls `eval_scenario` once with
the full scenario; no selected expression is compiled or evaluated separately.
The existing optional proof path inside `eval_scenario` remains unchanged.
Diagnostics always describe its initial reduction, excluding proof verification.

## Routes and feature

- `POST /api/v1/eval` accepts the existing scenario JSON. It now returns
  `mapStatus` and, when enabled, `costSpans` from that run alongside `values`,
  `hotSpots`, `trace`, `reducedTo`, `cost` and `costLimit`.
- `POST /api/v1/cost-spans` accepts the same scenario and returns the same
  envelope. It is registered only with `ergo-web`'s `cost-trace` feature **and**
  `AppConfig.cost_trace = true`. The feature defaults on and forwards to
  `ergo-sandbox/cost-trace`. Disabled or uncompiled routes return 404;
  `GET /api/v1/config` exposes the effective `costTrace` boolean.
- `POST /api/v1/explain` is feature-independent. Its body is
  `{"scenario":{"source":"sigmaProp(HEIGHT > 100)","height":200},"selection":{"offset":10,"length":6}}`.
  It returns the eval envelope from that same run, plus `message`,
  `selectionRule` and nullable `expression`.

`selection.offset` and `length` are UTF-8 bytes forming a nonempty half-open
interval within `source`, on character boundaries. Alternatively,
`{"line":1,"col":11}` selects an exact start point, using the pinned
compiler's one-based UTF-16 line/column convention (including its CRLF quirk).
Do not mix the forms. Write converts CodeMirror's UTF-16 selection into bytes.
It sends the full scenario, preserving its context; it refuses a scenario
containing a tree or a different source. For both Run and Explain, scenario parameters/network take
precedence when explicitly supplied; otherwise Write's current controls fill
those fields. Select one range, then click **Explain selection**.

## Source positions and selection

The shared `source_positions` step recompiles only to obtain compiler metadata,
checks byte identity with the evaluated tree, and checks `SourceMap.aligns_with`
against the canonical preorder walk of those evaluated bytes. It never reruns
the scenario. It also conservatively withholds citations whenever a quoted parameter name
could undergo string substitution; offsets into substituted text are not editor
citations. This can withhold an unchanged string too, rather than guess.
`mapStatus` is `aligned`, `misaligned`, `no-map`, `no-source`, or
`substituted-source`. No positions or explanations are attributed without a
usable map. Templates currently have no source map.

The compiler supplies **start positions only**: `{offset,line,col,kind:"start"}`.
There is no `end`. UI links select the token beginning there, not an inferred
expression extent. Byte offsets are authoritative; line/column values come
from `ergo_compiler::span::line_col` unchanged.

For a normal selection, choose the smallest source offset inside the interval,
then the smallest IR subtree among nodes at that offset (innermost), then
lowest preorder id. This matters when `HEIGHT` and its comparison share a
start. The rule is `first-start-in-selection-then-innermost`. It does not imply
that the selected text exactly matches the expression's extent.

The entire source (`offset:0,length:source byte length`) explicitly denotes
IR root 0, with rule `whole-contract-root`. A synthesized block or position-zero
root may have no compiler citation: its span stays null. A whole selection can
still show the run's `reducedTo` on the reducer's trivial fast path, even when
there is no recorded root value. A non-whole selection does not borrow this
exception and never guesses a neighbour. Unmapped names, whitespace, inlined
constructs and nodes without a recorded value return
`no evaluated expression starts in this selection`. Map failures have a
separate explicit message.

Every expression value carries its index in the response's full `values`
trace, and those values' IR ids identify the chosen expression. Multiple
records remain ordered; no last-value guess or second context is introduced.
The pinned value recorder identifies nodes by address; evaluator-made copies
(such as closure bodies or deserialisation rewrites) may have no records for
the original id. Such missing values stay missing.

## Cost ownership and reconciliation

`costSpans` contains `totalJit`, `attributedJit`, `ambiguousJit`,
`unattributedJit`, `exactShare`, `mapStatus`, and ranked `rows`. Each row has
`rawLabel`, readable `label`, `jit`, `count`, `share`, `rule`, `reason`, and
`candidates` with IR ids and nullable compiler start positions.

- `exact`: an explicit `OP:0xNN` or `Arith:0xNN` label has exactly one opcode
  node in the entire evaluated tree, with a compiler position.
- `ambiguous`: several opcode nodes are candidates. Every candidate is listed,
  including uncited nodes, and no cost is assigned to any individual candidate.
- `unattributed`: no uniquely citable opcode node, unavailable/misaligned map,
  a constant, a method/crypto/equality label without explicit opcode ownership,
  or a label whose origin cannot be bounded to that tree.

All nodes count, including branches not taken. Runtime details like `n=8`
remain separate labels but do not narrow candidates: there is no proven link
from that detail or evaluation order to a particular repeated IR opcode.

Two additional conservative exclusions follow the pinned engine's code:
`DeserializeContext`/`DeserializeRegister` can run opcodes outside the mapped
tree, so their containing trees receive no source cost attribution. Method
and property dispatch can reuse `0xC6`, `0xE3`, and `0x9B` charge labels for
register, context-variable and XOR helpers; those labels remain unattributed
when method/property nodes occur in the tree.

Some costs, notably inline constants, increase the cumulative trace total
without a labelled entry. Gaps before recorded entries become an explicit
`Unlabelled trace cost` row. Thus exact + ambiguous + unattributed equals the
last recorded cumulative trace total and the sum of row costs. `exactShare`
is exact / total (0 for an empty trace), not a claim of security coverage.
Errors can leave a partial trace: charges after its last entry are outside
this recorded total. It is distinct from the block-unit `cost`, which can
instead describe the later proof-verification pass. Write uses the eval
response directly; displaying cost never issues another evaluation request.

## Residuals and engine rendering limits

Values are the recorder's original strings, unchanged. The engine truncates
strings after 150 bytes and appends `...`; the response and UI mark that.
The whole-contract residual comes directly from the same run's `reducedTo`.
For a sub-expression, complete recorded sigma renderings are decoded only for
presentation and passed to `sigma_boolean_pretty`, the same printer used for
`reducedTo`. No proposition is inferred from truncated text. When a complete
residual is unavailable, its field stays null, its original recorded value
is shown, and `residualNote` explains the limit. The pinned engine is unchanged.

Editor, parameter and scenario changes invalidate diagnostics and pending
Explain responses. Generation checks prevent old Run results or old source
links from applying to the current editor. Rendered server/source text uses
text nodes, never HTML.
