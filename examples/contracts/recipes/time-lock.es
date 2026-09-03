/**
 * Savings you cannot touch until a date. Until then nobody can spend; after
 * it, only you can.
 * @param owner Who may spend after the lock passes? — An Ergo address from your wallet.
 * @param unlockHeight When should it unlock? — Nothing can be spent before this.
 */
@contract def timeLock(owner: SigmaProp, unlockHeight: Int) =
  owner && sigmaProp(HEIGHT >= unlockHeight)
