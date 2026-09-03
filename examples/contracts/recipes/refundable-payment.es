/**
 * Pay someone, and get it back if they have not claimed it by a date. The
 * receiver can take it at any time until then; after the date you can reclaim.
 * @param receiver Who are you paying? — Their Ergo address.
 * @param sender Who gets the refund? — Usually your own address.
 * @param refundHeight After which date may you reclaim it? — The receiver can claim until then.
 */
@contract def refundablePayment(receiver: SigmaProp, sender: SigmaProp, refundHeight: Int) =
  receiver || (sender && sigmaProp(HEIGHT >= refundHeight))
