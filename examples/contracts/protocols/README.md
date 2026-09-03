# Protocols

Multi-box systems written for this workbench, in the shape of contracts
that run on mainnet, with the arithmetic that makes them hard. Unlike the
recipes, these are for people who read ErgoScript: each is a `$param`
template you open in Write mode, and each ships with a **property-style
suite** whose expectations come from an independent model of the rules
(`examples/tests/gen/*.py`), swept over prices and amounts and pinned at
the one-unit boundaries — then run through the node's own reducer.

| Protocol | Files | What the suite sweeps |
|---|---|---|
| **AMM** (`amm/`) — a constant-product pool between ERG and one token, the shape of Spectrum's N2T pool in one script, plus a swap order that protects the trader from a bad fill | `pool.es`, `swap-order.es` | 8 swaps each way at the most the curve allows and one unit over; 6 deposits at the proportional LP and one over; 6 redeems at the proportional reserves and one over on each side; a swap that mints LP, a foreign script, a swapped NFT, a changed LP id, a two-token successor, a deposit that withdraws, no outputs (69 cases); the order filled at, above and below its minimum, in the wrong token, against a non-pool, and cancelled (6) |
| **Bank** (`bank/`) — a reserve-backed stablecoin bank, the shape of AgeUSD / SigmaUSD in one script, with an oracle data input, a fee, and the reserve-ratio window | `bank.es` | 18 random mint/redeem actions across six ERG prices at the exact price plus fee and one nanoERG short; registers not tracked; two actions at once; wrong or missing oracle; foreign script; an under-collateralised bank (SC price capped by the reserve, SC mint refused, SC redeem at the capped price, RC mint at the default price, RC redeem refused) and an over-collateralised one (RC mint refused, RC redeem allowed) (47 cases) |

Regenerate a suite with `python3 examples/tests/gen/amm.py` (or `bank.py`)
from the repo root, then `ergo-es test examples/tests/amm-pool.test.json`.
The models are deliberately plain integer arithmetic in the same order as
the script, so a disagreement between the two is a real finding about one
of them, not a rounding artefact.

What these do not do, on purpose: the pool's bootstrap (the first box is
created with its liquidity and LP already circulating), the bank's
update/ballot machinery, and any off-chain bot. They are the on-chain
invariants, which is the part a workbench can prove things about.
