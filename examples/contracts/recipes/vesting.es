/**
 * Release funds to someone gradually between two heights. Before the start
 * nothing can be taken; between start and end the beneficiary may withdraw
 * up to the vested share, leaving the rest in a box under this same
 * contract; after the end everything is theirs.
 * @param beneficiary the address the funds vest to
 * @param startHeight the block height at which vesting begins
 * @param endHeight the block height at which everything has vested
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
