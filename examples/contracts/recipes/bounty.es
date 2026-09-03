/**
 * Pay whoever can reveal a secret. You lock funds against the hash of a
 * secret phrase; anyone who presents the phrase itself (in their spending
 * transaction) can take the funds. You can reclaim after a deadline if
 * nobody does.
 * @param secretHash The hash of the secret — blake2b256 of the secret, as 64 hex characters (never the secret itself).
 * @param funder Who may reclaim after the deadline? — Your own address.
 * @param deadlineHeight Until when can the secret be claimed? — After this you may take the funds back.
 */
@contract def bounty(secretHash: Coll[Byte], funder: SigmaProp, deadlineHeight: Int) = {
  val revealed = getVar[Coll[Byte]](0)
  val claimed = revealed.isDefined && blake2b256(revealed.get) == secretHash && HEIGHT < deadlineHeight
  sigmaProp(claimed) || (funder && sigmaProp(HEIGHT >= deadlineHeight))
}
