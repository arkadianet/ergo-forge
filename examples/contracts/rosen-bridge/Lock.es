{
  val GuardNFT = fromBase64("GUARD_NFT");
  val GuardBox = CONTEXT.dataInputs(0);
  val paymentSignCount = GuardBox.R5[Coll[Int]].get(0);
  val signedColl = GuardBox.R4[Coll[Coll[Byte]]].get.map { (row: Coll[Byte]) => proveDlog(decodePoint(row)) };
  val verifyGuard = GuardBox.tokens.exists { (token: (Coll[Byte], Long)) => token._1 == GuardNFT };
  sigmaProp(
    allOf(
      Coll(
        verifyGuard,
        atLeast(paymentSignCount, signedColl)
      )
    )
  )
}
