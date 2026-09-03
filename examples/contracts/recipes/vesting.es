/**
 * Release funds to someone gradually between two dates. Before the start
 * nothing can be taken; in between they may withdraw the vested share, and
 * the rest stays locked; after the end everything is theirs.
 * @param beneficiary Who receives the funds? — Their Ergo address.
 * @param startHeight When does vesting start? — Nothing vests before this.
 * @param endHeight When has everything vested? — After this they may take it all.
 */
@contract def vesting(beneficiary: SigmaProp, startHeight: Int, endHeight: Int) = {
  val total = SELF.value
  val span = (endHeight - startHeight).toLong
  val elapsed = (HEIGHT - startHeight).toLong
  val fullyVested = HEIGHT >= endHeight
  // Vested share so far, in nanoERG (linear between start and end).
  val vested = if (fullyVested) total else if (HEIGHT < startHeight) 0L else total * elapsed / span
  val remainder = OUTPUTS(0)
  val keepsRest = remainder.propositionBytes == SELF.propositionBytes &&
                  remainder.value >= total - vested &&
                  remainder.tokens.size == 0 && remainder.R4[Int].isDefined == false
  beneficiary && sigmaProp(fullyVested || keepsRest)
}
