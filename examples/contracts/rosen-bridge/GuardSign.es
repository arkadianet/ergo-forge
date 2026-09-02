{
  // ----------------- REGISTERS
  // R4: Coll[Coll[Byte]] = [GUARD_PKS]
  // R5: Coll[Int] = [Payment Required Signs, Update Required Signs]
  // ----------------- TOKENS
  // 0: GuardNFT
  // --------------------
  // [GuardSign (sign atleast update required sign on it)] => [GuardSign]
  val guardBoxes = INPUTS.filter { (box: Box) => box.tokens.size > 0 && box.tokens(0)._1 == SELF.tokens(0)._1 }
  val signedColl = SELF.R4[Coll[Coll[Byte]]].get.map { (row: Coll[Byte]) => proveDlog(decodePoint(row)) }
  val updateSignCount = SELF.R5[Coll[Int]].get(1)
  val updateSignLeast = atLeast(updateSignCount, signedColl)
  sigmaProp(
    allOf(
      Coll(
        guardBoxes.size == 1,
        OUTPUTS(0).tokens(0)._1 == SELF.tokens(0)._1,
        OUTPUTS(0).tokens(0)._2 == SELF.tokens(0)._2,
        updateSignLeast
      )
    )
  )
}
