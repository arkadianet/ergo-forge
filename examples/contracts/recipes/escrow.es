/**
 * A payment held until two of three agree: the buyer and seller, or either
 * of them together with an arbiter who settles a dispute. Nobody can take
 * the money alone.
 * @param buyer Who is paying? — The buyer's Ergo address.
 * @param seller Who is being paid? — The seller's Ergo address.
 * @param arbiter Who settles disputes? — A third party both sides trust.
 */
@contract def escrow(buyer: SigmaProp, seller: SigmaProp, arbiter: SigmaProp) =
  atLeast(2, Coll(buyer, seller, arbiter))
