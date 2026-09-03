/**
 * Sell something to the highest bidder. You lock the item (a token) here
 * with a starting price. Anyone can bid by rebuilding this box with a
 * higher amount, which refunds the previous bidder in the same transaction.
 * Once the auction ends, anyone can settle it: you receive the winning bid
 * and the winner receives the item. If nobody bid, you can take the item
 * back.
 * @param seller Who is selling? — Your Ergo address; it receives the winning bid.
 * @param endHeight When does the auction end? — Bidding stops at this date; after it the sale settles.
 * @param minBidAmount Starting price? — The first bid must be at least this.
 * @param minRaiseAmount Minimum raise? — Each bid must beat the last by at least this.
 */
@contract def auction(seller: SigmaProp, endHeight: Int, minBidAmount: Long, minRaiseAmount: Long) = {
  // R4 holds whoever made the current bid; before any bid it is you (or empty).
  val bidder = SELF.R4[SigmaProp].getOrElse(seller)
  val noBids = bidder.propBytes == seller.propBytes
  val floor = if (noBids) minBidAmount else SELF.value + minRaiseAmount
  val bid = HEIGHT < endHeight && OUTPUTS.size > 1 &&
    OUTPUTS(0).propositionBytes == SELF.propositionBytes &&
    OUTPUTS(0).tokens == SELF.tokens &&
    OUTPUTS(0).value >= floor &&
    OUTPUTS(0).R4[SigmaProp].isDefined &&
    OUTPUTS(1).propositionBytes == bidder.propBytes &&
    OUTPUTS(1).value >= SELF.value
  val settle = HEIGHT >= endHeight && (!noBids) && OUTPUTS.size > 1 &&
    OUTPUTS(0).propositionBytes == seller.propBytes &&
    OUTPUTS(0).value >= SELF.value &&
    OUTPUTS(1).propositionBytes == bidder.propBytes &&
    OUTPUTS(1).tokens == SELF.tokens
  sigmaProp(bid || settle) || (seller && sigmaProp(noBids))
}
