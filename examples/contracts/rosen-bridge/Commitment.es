{
  // ----------------- REGISTERS
  // R4: Coll[Byte] = WID
  // R5: Coll[Byte] = Event ID (Hash(TxId))
  // R6: Coll[Byte] = Event Data Digest
  // R7: Coll[Byte] = Permit Script Digest
  // ----------------- TOKENS
  // 0: X-RWT

  val eventTriggerHash = fromBase64("EVENT_TRIGGER_SCRIPT_HASH");
  val RWTRepoNFT = fromBase64("RWT_REPO_NFT");
  val repoConfigNft = fromBase64("REPO_CONFIG_NFT");
  val trigger = if (blake2b256(INPUTS(0).propositionBytes) == eventTriggerHash) INPUTS(0) else OUTPUTS(0)
  val myWID = SELF.R4[Coll[Byte]].get
  val eventData = trigger.R5[Coll[Coll[Byte]]].get.fold(Coll[Byte](), {(a: Coll[Byte], b: Coll[Byte]) => a ++ b })
  if(blake2b256(INPUTS(0).propositionBytes) == eventTriggerHash){
    // Reward Distribution (for missed commitments)
    // [EventTrigger, Commitments[], BridgeWallet] => [WatcherPermits[], BridgeWallet]
    val WIDs = OUTPUTS.filter{(box:Box) 
        => box.tokens.size > 0 && box.tokens(0)._1 == SELF.tokens(0)._1
      }
      .slice(0, trigger.R7[Int].get)
      .map{(box:Box) => box.R4[Coll[Byte]].get}
    val commitmentsWithMyWid = INPUTS.filter{(box:Box) =>
        box.tokens.size > 0 &&
        box.tokens(0)._1 == SELF.tokens(0)._1 && 
        box.propositionBytes == SELF.propositionBytes &&
        box.R4[Coll[Byte]].get == myWID
      }
    val permitBox = OUTPUTS.filter {(box:Box) =>
      box.R4[Coll[Byte]].isDefined &&
      box.R4[Coll[Byte]].get == myWID
    }(0)
    val WIDExists =  WIDs.exists {(WID: Coll[Byte]) => myWID == WID}
    sigmaProp(
      allOf(
        Coll(
          blake2b256(permitBox.propositionBytes) == SELF.R7[Coll[Byte]].get,
          permitBox.tokens(0)._1 == SELF.tokens(0)._1,
          permitBox.tokens(0)._2 == SELF.tokens(0)._2,
          // check for duplicates
          WIDExists == false,
          commitmentsWithMyWid.size == 1,
          // validate commitment
          blake2b256(eventData ++ myWID) == SELF.R6[Coll[Byte]].get
        )
      )
    )

  } else if (blake2b256(OUTPUTS(0).propositionBytes) == eventTriggerHash){
    // Event Trigger Creation
    // [Commitments[]] + [(DataInput) RepoConfigBox + Repo] => [EventTrigger]
    val commitmentBoxes = INPUTS.filter{ 
      (box: Box) => 
        SELF.propositionBytes == box.propositionBytes && 
        box.tokens.size > 0 && 
        box.tokens(0)._1 == SELF.tokens(0)._1 
      }
    val WIDs = commitmentBoxes.map{(box:Box) => box.R4[Coll[Byte]].get}
    val widListDigest = blake2b256(WIDs.fold(Coll[Byte](), {(a: Coll[Byte], b: Coll[Byte]) => a++b}))
    val myWIDCommitments = commitmentBoxes.filter{ (box: Box) => box.R4[Coll[Byte]].get == myWID }
    val EventBoxErgs = commitmentBoxes.map { (box: Box) => box.value }.fold(0L, { (a: Long, b: Long) => a + b })
    val repoConfigBox = CONTEXT.dataInputs(0)
    val repoConfig = repoConfigBox.R4[Coll[Long]].get
    val repo = CONTEXT.dataInputs(1)
    val watcherCount = repo.R5[Long].get
    val eventId = blake2b256(trigger.R5[Coll[Coll[Byte]]].get(0))
    val maxCommitment = repoConfig(3)
    val requiredCommitmentFromFormula: Long = repoConfig(2) + repoConfig(1) * watcherCount / 100L
    val requiredCommitment = if(maxCommitment < requiredCommitmentFromFormula) {
      maxCommitment
    } else {
      requiredCommitmentFromFormula
    }
    sigmaProp(
      allOf(
        Coll(
          //check repo
          repoConfigBox.tokens(0)._1 == repoConfigNft,
          repo.tokens(0)._1 == RWTRepoNFT,
          repo.tokens(1)._1 == SELF.tokens(0)._1,
          // prevent duplicate commitments
          myWIDCommitments.size == 1,
          // verify trigger params
          trigger.value >= EventBoxErgs,
          trigger.R6[Coll[Byte]].get == SELF.R7[Coll[Byte]].get,
          trigger.R7[Int].get == commitmentBoxes.size,
          trigger.R4[Coll[Byte]].get == widListDigest,
          // verify commitment to be correct
          blake2b256(eventData ++ myWID) == SELF.R6[Coll[Byte]].get,
          // check event id
          SELF.R5[Coll[Byte]].get == eventId,
          // check commitment count
          commitmentBoxes.size > requiredCommitment,
          // Check required RWT
          SELF.tokens(0)._2 == repoConfig(0),
          trigger.tokens(0)._2 == repoConfig(0) * commitmentBoxes.size,
          trigger.tokens(0)._1 == SELF.tokens(0)._1
        )
      )
    )
  } else {
    // Commitment Redeem
    // [Commitment, WID] => [Permit, WID]
    sigmaProp(
      allOf(
        Coll(
          SELF.id == INPUTS(0).id,
          OUTPUTS(0).tokens(0)._1 == SELF.tokens(0)._1,
          OUTPUTS(0).tokens(0)._2 == SELF.tokens(0)._2,
          // check WID copied
          OUTPUTS(0).R4[Coll[Byte]].get == myWID,
          // check user WID
          OUTPUTS(1).tokens(0)._1 == myWID,
          // check permit contract address
          blake2b256(OUTPUTS(0).propositionBytes) == SELF.R7[Coll[Byte]].get
        )
      )
    )
  }
}
