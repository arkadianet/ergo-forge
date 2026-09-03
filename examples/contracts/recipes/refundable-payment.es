/**
 * Pay someone, and get it back if they have not claimed it by a height.
 * The receiver can spend at any time; the sender can reclaim after the
 * deadline.
 * @param receiver the address being paid
 * @param sender the address that may reclaim after the deadline
 * @param refundHeight the block height from which the sender may reclaim
 */
@contract def refundablePayment(receiver: SigmaProp, sender: SigmaProp, refundHeight: Int) =
  receiver || (sender && sigmaProp(HEIGHT >= refundHeight))
