`parameters.json` records the four Phoenix/Dexy structural parameter sets from
the baseline `audit_sweep` run. These are synthetic test values, not deployment
parameters. Their source contracts remain in `examples/contracts`.

A third discharge fixture, covering a pre-launch protocol whose contracts bind a
config box by NFT in a companion contract, is deliberately held back until that
protocol has been told what we found. A public fixture naming a protocol is a
disclosure signal in itself, whatever it asserts.
