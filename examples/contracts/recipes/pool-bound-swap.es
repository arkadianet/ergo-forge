/**
 * Authorize a swap against one named pool, identifying it before doing reserve arithmetic.
 * Walkthrough: put the pool at INPUTS(0) and this authorizer at INPUTS(1).
 * Return the pool at OUTPUTS(0) and this authorizer at OUTPUTS(1), keeping
 * the pool singleton NFT. The reserve product may only grow; pool token type,
 * pool script and authorizer value and tokens are preserved. Exactly two
 * outputs are allowed in this teaching shape. Try a junk-token decoy at
 * INPUTS(0): matching arithmetic must still fail the pool identity check.
 * The suites are synthetic reductions, not complete signed transactions.
 * @param poolNFT Which pool? — Its singleton NFT id, held in token slot zero.
 */
@contract def poolBoundSwap(poolNFT: Coll[Byte]) = {
  if (INPUTS.size >= 2 && OUTPUTS.size == 2) {
    val pool = INPUTS(0)
    val next = OUTPUTS(0)
    val order = OUTPUTS(1)
    if (pool.tokens.size == 2 && next.tokens.size == 2) {
      val identified = pool.tokens(0)._1 == poolNFT
      sigmaProp(identified &&
        pool.tokens(0)._2 == 1L && next.tokens(0)._1 == poolNFT && next.tokens(0)._2 == 1L &&
        next.propositionBytes == pool.propositionBytes &&
        next.tokens(1)._1 == pool.tokens(1)._1 &&
        next.value.toBigInt * next.tokens(1)._2.toBigInt >= pool.value.toBigInt * pool.tokens(1)._2.toBigInt &&
        order.propositionBytes == SELF.propositionBytes &&
        order.value >= SELF.value && order.tokens == SELF.tokens)
    } else sigmaProp(false)
  } else sigmaProp(false)
}
