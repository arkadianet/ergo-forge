// USE LP swap — FIXED shape (regression fixture for the 2026-09-08 drain).
//
// This is the deployed useLpSwap logic, decompiled from mainnet swap-NFT box
// ef461517a55b8bfcd30356f112928f3333b5b50faf472e8374081307a09110cf, with the
// ONE missing check restored: the box the swap does its constant-product math
// on must carry the LP singleton NFT. The deployed contract read the pool
// positionally as INPUTS(0)/OUTPUTS(0) and never asserted which box that was,
// so an attacker put a decoy at index 0 and drained the real pool spent
// alongside it. Binding by the singleton NFT closes the substitution: only the
// genuine pool holds that token.
//
// $lpNft: Coll[Byte]  -- the LP pool's singleton NFT (tokens(0) of the pool)
{
  val poolIn      = INPUTS(0)
  val poolInToks  = poolIn.tokens
  val poolOut     = OUTPUTS(0)
  val poolOutToks = poolOut.tokens

  // THE FIX: bind the pool the maths runs on by its singleton NFT, at both the
  // spent box and its successor. A decoy cannot hold the unique LP token.
  val poolBound =
    poolInToks(0)._1  == $lpNft &&
    poolOutToks(0)._1 == $lpNft

  val ergIn      = poolIn.value
  val ergDelta   = poolOut.value - ergIn
  val tokenInY   = poolInToks(2)._2
  val tokenDelta = poolOutToks(2)._2 - tokenInY
  val swapSucc   = OUTPUTS(1)

  val validSwap =
    (poolInToks(1)._2 - poolOutToks(1)._2 == 0L) &&
    (if (ergDelta > 0L)
        tokenInY.toBigInt * ergDelta.toBigInt * 997.toBigInt >=
          (-tokenDelta).toBigInt *
            (ergIn.toBigInt * 1000.toBigInt + ergDelta.toBigInt * 997.toBigInt)
     else
        ergIn.toBigInt * tokenDelta.toBigInt * 997.toBigInt >=
          (-ergDelta).toBigInt *
            (tokenInY.toBigInt * 1000.toBigInt + tokenDelta.toBigInt * 997.toBigInt)) &&
    swapSucc.propositionBytes == SELF.propositionBytes &&
    swapSucc.value >= SELF.value &&
    swapSucc.tokens == SELF.tokens

  sigmaProp(poolBound && validSwap)
}
