{
  // ----------------- REGISTERS
  // R4: Coll[Byte] = Chain id
  // R5: Long = total watchers
  // ----------------- TOKENS
  // 0: X-RWT Repo NFT
  // 1: X-RWT
  // 2: RSN
  // 3: X-AWC NFT

  val repoConfigNft = fromBase64("REPO_CONFIG_NFT");
  val watcherCollateralScriptHash = fromBase64("WATCHER_COLLATERAL_SCRIPT_HASH");
  if(OUTPUTS(0).tokens(0)._1 == repoConfigNft){
    // RWT Repo Update transaction
    sigmaProp(true)
  } else {
    val permitScriptHash = fromBase64("PERMIT_SCRIPT_HASH");
    val repoOut = OUTPUTS(0)
    val repo = SELF
    val repoReplication = allOf(
      Coll(
        repoOut.propositionBytes == repo.propositionBytes,
        repoOut.value >= repo.value,
        repoOut.tokens(0)._1 == repo.tokens(0)._1,
        repoOut.tokens(0)._2 == repo.tokens(0)._2,
        repoOut.tokens(1)._1 == repo.tokens(1)._1,
        repoOut.tokens(2)._1 == repo.tokens(2)._1,
        repoOut.tokens(3)._1 == repo.tokens(3)._1,
        repoOut.R4[Coll[Byte]].get == repo.R4[Coll[Byte]].get,
      )
    )
    if(repo.tokens(1)._2 > repoOut.tokens(1)._2){
      // Getting Watcher Permit
      val outCollateral = OUTPUTS(1)
      val permit = OUTPUTS(2)
      val outWIDBox = OUTPUTS(3)
      val RWTOut = repo.tokens(1)._2 - repoOut.tokens(1)._2
      val permitCreation = allOf(
        Coll(
          repoReplication,
          RWTOut == repoOut.tokens(2)._2 - repo.tokens(2)._2,
          permit.tokens(0)._2 == RWTOut,
          permit.tokens(0)._1 == SELF.tokens(1)._1,
          blake2b256(permit.propositionBytes) == permitScriptHash,
        )
      )
      if(repoOut.tokens(3)._2 == repo.tokens(3)._2 - 1){
        // Getting initial permit
        // [Repo, UserInputs] + [(DataInput) RepoConfig] => [Repo, Collateral, watcherPermit, WIDBox]
        val repoConfigBox = CONTEXT.dataInputs(0)
        val repoConfig = repoConfigBox.R4[Coll[Long]]
        sigmaProp(
          allOf(
            Coll(
              repoOut.R5[Long].get == repo.R5[Long].get + 1,
              // Permit and WID checks
              permitCreation,
              permit.R4[Coll[Byte]].get == repo.id,
              outWIDBox.tokens(0)._1 == repo.id,
              outWIDBox.tokens(0)._2 >= 3,
              // Repo config checks
              repoConfigBox.tokens(0)._1 == repoConfigNft,
              // Collateral checks
              blake2b256(outCollateral.propositionBytes) == watcherCollateralScriptHash,
              outCollateral.R4[Coll[Byte]].get == repo.id,
              outCollateral.value >= repoConfig.get(4),
              outCollateral.R5[Long].get == RWTOut,
              outCollateral.tokens(0)._1 == repo.tokens(3)._1,
              if(repoConfig.get(5) > 0){
                allOf(
                  Coll(
                    outCollateral.tokens(1)._1 == repo.tokens(2)._1,
                    outCollateral.tokens(1)._2 >= repoConfig.get(5)
                  )
                )
              }else{
                true
              }
            )
          )
        )
      } else {
        // Extending Permit
        // [Repo, Collateral, WIDBox] => [Repo, Collateral, watcherPermit, WIDBox]
        val collateral = INPUTS(1)
        val WID = outCollateral.R4[Coll[Byte]].get
        sigmaProp(
          allOf(
            Coll(
              // Permit check
              permitCreation,
              permit.R4[Coll[Byte]].get == WID,
              // Rwt repo check
              repoOut.tokens(3)._2 == repo.tokens(3)._2,
              repoOut.R5[Long].get == repo.R5[Long].get,
              // Collateral check
              outCollateral.tokens(0)._1 == repo.tokens(3)._1,
              collateral.R5[Long].get + RWTOut == outCollateral.R5[Long].get
            )
          )
        )
      }
    }else{
      // Returning Watcher Permit
      val permit = INPUTS(2)
      val RWTIn = repoOut.tokens(1)._2 - repo.tokens(1)._2
      val collateral = INPUTS(1)
      val validateUpdates = if(collateral.R5[Long].get > RWTIn) {
        // two scenarios:
        //   - partial return permit
        //   [Repo, Collateral, Permit, WIDBox, Permits(Optional)] => [Repo, Collateral, Permit(Optional), WIDBox]
        //   - slash
        //   [Repo, Collateral, Fraud, Cleanup] => [Repo, Collateral, Cleanup]
        val outCollateral = OUTPUTS(1)
        allOf(
          Coll(
            repo.tokens(3)._2 == repoOut.tokens(3)._2,
            repoOut.R5[Long].get == repo.R5[Long].get,
            collateral.R5[Long].get - RWTIn == outCollateral.R5[Long].get,
          )
        )
      }else{
        // Returning total permit
        // [Repo, Collateral, Permit, WIDBox] => [Repo, UserChange(+Collateral)]
        allOf(
          Coll(
            repoOut.tokens(3)._2 == repo.tokens(3)._2 + 1,
            repoOut.R5[Long].get == repo.R5[Long].get - 1,
          )
        )
      }
      val WID = collateral.R4[Coll[Byte]].get
      sigmaProp(
        allOf(
          Coll(
            repoReplication,
            permit.R4[Coll[Byte]].get == WID,
            permit.tokens(0)._1 == repo.tokens(1)._1,
            RWTIn == repo.tokens(2)._2 - repoOut.tokens(2)._2,
            validateUpdates,
            collateral.tokens(0)._1 == repo.tokens(3)._1,
          )
        )
      )
    }
  }
}
