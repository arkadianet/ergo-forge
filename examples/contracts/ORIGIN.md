# Example contracts

`basics/`, `recipes/` and `protocols/` are written for this playground.
`protocols/` are multi-box systems (an AMM pool with a swap order, a
reserve-backed stablecoin bank) with property-style suites generated from
independent models in `examples/tests/gen/`; see `protocols/README.md`. `recipes/` are
EIP-5 `@contract def` templates whose doc block and `@param` lines are
written as questions for non-technical users: the Build mode is a form over
them. Every recipe with state or more than one spending path ships with a test
suite in `examples/tests/` that pins each path — run `ergo-es test` on it.
Writing those suites is how the recipes were found correct (the
subscription's last-payment path indexed a second output that need not
exist until its suite said so).

Every other directory is a **real, deployed ErgoScript source**, vendored
verbatim from the Ergo node's parser corpus
(`arkadianet/ergo` · `test-vectors/ergoscript/corpus`, whose `MANIFEST.md`
maps each file to its upstream project: Dexy, HodlCoin, ChainCash / Basis,
Rosen Bridge, CrystalPool, Curve Trees, and the ErgoScript LSP examples).
They are here so the playground opens on the kind of source people actually
write — parameterised with `$name` constants and, in the case of the EIP-5
`@contract def` files, templates. The `lsp/` and EIP-5 files are examples
from tooling repositories, not deployed contracts.

Copyright remains with the upstream projects; see each project's licence.
Nothing is edited.
