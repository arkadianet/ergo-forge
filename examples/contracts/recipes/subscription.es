/**
 * Pay someone a fixed amount at most once every so many blocks, from a
 * pot you fund up front. The receiver claims one payment per period; the
 * rest stays in the pot. You can take back whatever is left at any time.
 * @param receiver Who gets paid? — Their Ergo address.
 * @param funder Who funded the pot and may cancel? — Your own address.
 * @param amount How much per payment, in nanoERG — 1 ERG = 1,000,000,000 nanoERG.
 * @param periodBlocks How many blocks between payments? — About 720 per day, 21,600 per month.
 */
@contract def subscription(receiver: SigmaProp, funder: SigmaProp, amount: Long, periodBlocks: Int) = {
  // R4 of the pot records the height of the last claim (absent = never).
  val lastClaim = SELF.R4[Int].getOrElse(0)
  val due = HEIGHT >= lastClaim + periodBlocks
  val payment = OUTPUTS(0)
  val paidReceiver = payment.propositionBytes == receiver.propBytes && payment.value >= amount
  val potKept = OUTPUTS.size > 1 &&
                OUTPUTS(1).propositionBytes == SELF.propositionBytes &&
                OUTPUTS(1).value >= SELF.value - amount &&
                OUTPUTS(1).R4[Int].getOrElse(0) == HEIGHT
  val lastPayment = SELF.value <= amount && payment.propositionBytes == receiver.propBytes && payment.value >= SELF.value
  funder || (receiver && sigmaProp(due && ((paidReceiver && potKept) || lastPayment)))
}
