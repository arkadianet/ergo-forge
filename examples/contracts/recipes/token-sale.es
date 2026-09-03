/**
 * Sell tokens at a fixed price. Buyers may take some or all of the tokens
 * as long as you are paid for what they take; unsold tokens stay for sale.
 * You can cancel and take everything back at any time.
 * @param seller Who receives the payments? — Your Ergo address.
 * @param tokenId Which token are you selling? — The token id (64 characters), from your wallet or an explorer.
 * @param pricePerToken Price per token, in nanoERG — 1 ERG = 1,000,000,000 nanoERG.
 */
@contract def tokenSale(seller: SigmaProp, tokenId: Coll[Byte], pricePerToken: Long) = {
  val forSale = SELF.tokens(0)
  val stock = if (forSale._1 == tokenId) forSale._2 else 0L
  val payment = OUTPUTS(0)
  val restStock =
    if (OUTPUTS.size > 1 && OUTPUTS(1).tokens.size > 0 && OUTPUTS(1).tokens(0)._1 == tokenId) OUTPUTS(1).tokens(0)._2
    else 0L
  val sold = stock - restStock
  val paid = payment.propositionBytes == seller.propBytes && payment.value >= sold * pricePerToken
  val restKept = OUTPUTS.size > 1 &&
                 OUTPUTS(1).propositionBytes == SELF.propositionBytes &&
                 OUTPUTS(1).value >= SELF.value
  val partial = sold > 0L && sold < stock && paid && restKept
  val soldOut = sold == stock && stock > 0L && paid
  seller || sigmaProp(partial || soldOut)
}
