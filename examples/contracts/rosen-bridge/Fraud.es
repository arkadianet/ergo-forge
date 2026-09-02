{
  // ----------------- REGISTERS
  // R4: Coll[Byte] = WID
  // ----------------- TOKENS
  // 0: X-RWT

  val RWTRepoNFT = fromBase64("RWT_REPO_NFT");
  val cleanupNFT = fromBase64("CLEANUP_NFT");
  // RSN Slash
  // [Repo, Collateral, Fraud, Cleanup] => [Repo, Collateral, Cleanup, Slashed]
  val transferedRwt = OUTPUTS(0).tokens(1)._2 - INPUTS(0).tokens(1)._2
  sigmaProp(
    allOf(
      Coll(
        SELF.tokens(0)._2 == transferedRwt,
        SELF.id == INPUTS(2).id,
        INPUTS(0).tokens(0)._1 == RWTRepoNFT,
        INPUTS(3).tokens(0)._1 == cleanupNFT,
      )
    )
  )
}
