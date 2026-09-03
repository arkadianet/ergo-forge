/**
 * Sell tokens from this box at a fixed price. A buyer may take some or all
 * of the tokens as long as the seller is paid the price for what is taken
 * (first output); any tokens left must stay in a box under this same
 * contract (second output). The seller can always take everything back.
 * @param seller the address that receives the payment and may cancel
 * @param tokenId the id of the token being sold
 * @param pricePerToken the price of one token, in nanoERG
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
