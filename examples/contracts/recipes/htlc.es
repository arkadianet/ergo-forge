/**
 * Swap with someone on another chain (a hashed time lock). You lock funds
 * for a receiver, who can take them only by revealing a secret whose
 * SHA-256 hash you both agreed on — the same secret unlocks their side of
 * the trade. If they never reveal it, you take the funds back after the
 * deadline.
 * @param receiver Who may claim with the secret? — Their Ergo address.
 * @param sender Who takes the funds back after the deadline? — Your own address.
 * @param secretHashSha256 The SHA-256 hash of the secret — 64 hex characters; the secret itself stays private until the claim.
 * @param refundHeight When can you take the funds back? — Claims stop at this date.
 */
@contract def htlc(receiver: SigmaProp, sender: SigmaProp, secretHashSha256: Coll[Byte], refundHeight: Int) = {
  val revealed = getVar[Coll[Byte]](0)
  val claim = receiver && sigmaProp(revealed.isDefined && sha256(revealed.get) == secretHashSha256 && HEIGHT < refundHeight)
  val refund = sender && sigmaProp(HEIGHT >= refundHeight)
  claim || refund
}
