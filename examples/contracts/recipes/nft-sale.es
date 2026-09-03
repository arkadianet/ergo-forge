/**
 * Sell a token (an NFT) at a fixed price, with a royalty for its creator.
 * Anyone who pays you the price and the creator the royalty in the same
 * transaction gets the token. You can cancel and take it back at any time.
 * @param seller Who is selling? — Your Ergo address; it receives the price minus the royalty.
 * @param artist Who created the token? — Their Ergo address; it receives the royalty on this sale.
 * @param priceAmount Price? — What the buyer pays in total; the royalty comes out of it.
 * @param royaltyPercent Royalty for the creator, as a percentage — A whole number from 0 to 100.
 */
@contract def nftSale(seller: SigmaProp, artist: SigmaProp, priceAmount: Long, royaltyPercent: Int) = {
  val royalty = priceAmount * royaltyPercent.toLong / 100L
  val paidSeller = OUTPUTS.size > 0 && OUTPUTS(0).propositionBytes == seller.propBytes && OUTPUTS(0).value >= priceAmount - royalty
  val paidArtist = OUTPUTS.size > 1 && OUTPUTS(1).propositionBytes == artist.propBytes && OUTPUTS(1).value >= royalty
  // The buyer chooses where the token goes: the third output must carry it.
  val delivered = OUTPUTS.size > 2 && OUTPUTS(2).tokens == SELF.tokens
  seller || sigmaProp(paidSeller && paidArtist && delivered)
}
