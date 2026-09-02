# Example contracts

`basics/` are written for this playground.

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
