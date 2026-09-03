// A reserve-backed stablecoin bank (the shape of AgeUSD / SigmaUSD, in one
// script). The bank box holds:
//   value      the ERG reserve
//   tokens(0)  the bank NFT
//   tokens(1)  stablecoins (SC) not yet in circulation
//   tokens(2)  reserve coins (RC) not yet in circulation
//   R4[Long]   SC in circulation      R5[Long]   RC in circulation
// The oracle box (data input 0, identified by its NFT) reports in R4 the
// price of one SC unit in nanoERG. A spend rebuilds the bank as OUTPUTS(0)
// and does exactly one of: mint SC, redeem SC, mint RC, redeem RC, paying
// or receiving the nominal price plus a fee, subject to the reserve ratio
// after the action: minting SC and redeeming RC need it at or above the
// minimum; minting RC needs it at or below the maximum; redeeming SC is
// always allowed. SC price is the oracle rate, capped by what the reserve
// can cover; RC price is the equity per RC, or a default when there is no
// equity or no RC.
//
// $oracleNft: Coll[Byte]
// $minRatioPercent: Int     -- 400 in SigmaUSD
// $maxRatioPercent: Int     -- 800 in SigmaUSD
// $feePercent: Int          -- 2 in SigmaUSD
// $rcDefaultPrice: Long     -- nanoERG per RC when the equity says nothing
{
  val wellFormed = OUTPUTS.size > 0 && OUTPUTS(0).tokens.size == 3 && SELF.tokens.size == 3 &&
    CONTEXT.dataInputs.size > 0 && CONTEXT.dataInputs(0).tokens.size > 0 &&
    CONTEXT.dataInputs(0).tokens(0)._1 == $oracleNft &&
    CONTEXT.dataInputs(0).R4[Long].isDefined &&
    SELF.R4[Long].isDefined && SELF.R5[Long].isDefined &&
    OUTPUTS(0).R4[Long].isDefined && OUTPUTS(0).R5[Long].isDefined
  if (!wellFormed) sigmaProp(false) else {
    val successor = OUTPUTS(0)
    val rate = CONTEXT.dataInputs(0).R4[Long].get
    val preserved = successor.propositionBytes == SELF.propositionBytes &&
      successor.tokens(0) == SELF.tokens(0) &&
      successor.tokens(1)._1 == SELF.tokens(1)._1 &&
      successor.tokens(2)._1 == SELF.tokens(2)._1

    val reserveIn = SELF.value
    val reserveOut = successor.value
    val scCircIn = SELF.R4[Long].get
    val rcCircIn = SELF.R5[Long].get
    val scCircOut = successor.R4[Long].get
    val rcCircOut = successor.R5[Long].get
    val deltaSc = SELF.tokens(1)._2 - successor.tokens(1)._2
    val deltaRc = SELF.tokens(2)._2 - successor.tokens(2)._2
    val tracked = scCircOut == scCircIn + deltaSc && rcCircOut == rcCircIn + deltaRc &&
      scCircOut >= 0L && rcCircOut >= 0L
    val oneAction = (deltaSc != 0L && deltaRc == 0L) || (deltaSc == 0L && deltaRc != 0L)

    val scPrice = if (scCircIn == 0L) rate else min(rate, reserveIn / scCircIn)
    val equity = reserveIn - scCircIn * scPrice
    val rcPrice = if (rcCircIn > 0L && equity > 0L) equity / rcCircIn else $rcDefaultPrice

    val amount = if (deltaSc != 0L) deltaSc * scPrice else deltaRc * rcPrice
    val absAmount = if (amount < 0L) -amount else amount
    val fee = absAmount * $feePercent.toLong / 100L
    val deltaReserve = reserveOut - reserveIn
    val minting = amount > 0L
    val paid = if (minting) deltaReserve >= absAmount + fee else deltaReserve >= -(absAmount - fee)

    // Ratio after, in percent: reserveOut * 100 / (scCircOut * rate); no SC
    // out means "at the maximum" so RC can still be minted.
    val ratioOk =
      if (deltaSc > 0L) reserveOut.toBigInt * 100L.toBigInt >= scCircOut.toBigInt * rate.toBigInt * $minRatioPercent.toBigInt
      else if (deltaSc < 0L) true
      else if (deltaRc > 0L) scCircOut == 0L || reserveOut.toBigInt * 100L.toBigInt <= scCircOut.toBigInt * rate.toBigInt * $maxRatioPercent.toBigInt
      else scCircOut == 0L || reserveOut.toBigInt * 100L.toBigInt >= scCircOut.toBigInt * rate.toBigInt * $minRatioPercent.toBigInt

    sigmaProp(preserved && tracked && oneAction && paid && ratioOk)
  }
}
