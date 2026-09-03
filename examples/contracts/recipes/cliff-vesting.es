/**
 * Release funds to someone gradually, but nothing at all before a cliff.
 * Before the cliff nothing can be taken; from the cliff the share vested
 * since the start becomes available; after the end everything is theirs.
 * @param beneficiary Who receives the funds? — Their Ergo address.
 * @param startHeight When does vesting start counting? — Usually the day the funds are locked.
 * @param cliffHeight When can the first withdrawal happen? — Nothing at all before this date.
 * @param endHeight When has everything vested? — After this they may take it all.
 */
@contract def cliffVesting(beneficiary: SigmaProp, startHeight: Int, cliffHeight: Int, endHeight: Int) = {
  val total = SELF.value
  val span = (endHeight - startHeight).toLong
  val elapsed = (HEIGHT - startHeight).toLong
  val fullyVested = HEIGHT >= endHeight
  val vested = if (fullyVested) total else if (HEIGHT < cliffHeight) 0L else total * elapsed / span
  val remainder = OUTPUTS(0)
  val keepsRest = remainder.propositionBytes == SELF.propositionBytes &&
                  remainder.value >= total - vested &&
                  remainder.tokens.size == 0
  beneficiary && sigmaProp(fullyVested || keepsRest)
}
