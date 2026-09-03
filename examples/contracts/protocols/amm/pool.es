// A constant-product liquidity pool between ERG and one token (the shape
// of Spectrum's N2T pool, in one script). The pool box holds:
//   value      the ERG reserve (X)
//   tokens(0)  the pool NFT, which identifies this pool
//   tokens(1)  LP tokens not yet in circulation
//   tokens(2)  the token reserve (Y)
// Every spend must rebuild the box as OUTPUTS(0) with the same script, the
// same NFT and token ids, and do exactly one of: swap (LP unchanged, the
// product of the reserves may not fall after the fee), deposit (LP minted
// is at most the proportional share of what was added), redeem (what is
// taken out is at most the proportional share of the LP burned). The pool
// is bootstrapped by creating the box with its first liquidity and the
// matching LP already in circulation; the script never sees supply zero.
// Price protection for the trader lives in the order contract, not here.
//
// $lpSupply: Long  -- LP tokens minted at bootstrap: circulating = lpSupply - tokens(1)
// $feeNum: Int     -- 997 with feeDenom 1000 is a 0.3% fee
// $feeDenom: Int
{
  val wellFormed = OUTPUTS.size > 0 && OUTPUTS(0).tokens.size == 3 && SELF.tokens.size == 3
  if (!wellFormed) sigmaProp(false) else {
    val successor = OUTPUTS(0)
    val nftIn = SELF.tokens(0)
    val lpIn = SELF.tokens(1)
    val yIn = SELF.tokens(2)
    val nftOut = successor.tokens(0)
    val lpOut = successor.tokens(1)
    val yOut = successor.tokens(2)
    val preserved = successor.propositionBytes == SELF.propositionBytes &&
      nftOut == nftIn && lpOut._1 == lpIn._1 && yOut._1 == yIn._1

    val supplyIn = $lpSupply - lpIn._2
    val reservesXIn = SELF.value
    val reservesYIn = yIn._2
    val deltaSupply = lpIn._2 - lpOut._2
    val deltaX = successor.value - reservesXIn
    val deltaY = yOut._2 - reservesYIn

    val validSwap = deltaSupply == 0L && (
      if (deltaX > 0L)
        reservesYIn.toBigInt * deltaX.toBigInt * $feeNum.toBigInt >= -deltaY.toBigInt * (reservesXIn.toBigInt * $feeDenom.toBigInt + deltaX.toBigInt * $feeNum.toBigInt)
      else
        reservesXIn.toBigInt * deltaY.toBigInt * $feeNum.toBigInt >= -deltaX.toBigInt * (reservesYIn.toBigInt * $feeDenom.toBigInt + deltaY.toBigInt * $feeNum.toBigInt)
    )
    val validDeposit = deltaSupply > 0L && deltaX > 0L && deltaY > 0L &&
      deltaSupply.toBigInt * reservesXIn.toBigInt <= deltaX.toBigInt * supplyIn.toBigInt &&
      deltaSupply.toBigInt * reservesYIn.toBigInt <= deltaY.toBigInt * supplyIn.toBigInt
    val validRedeem = deltaSupply < 0L && deltaX < 0L && deltaY < 0L &&
      (-deltaX).toBigInt * supplyIn.toBigInt <= (-deltaSupply).toBigInt * reservesXIn.toBigInt &&
      (-deltaY).toBigInt * supplyIn.toBigInt <= (-deltaSupply).toBigInt * reservesYIn.toBigInt

    sigmaProp(preserved && (validSwap || validDeposit || validRedeem))
  }
}
