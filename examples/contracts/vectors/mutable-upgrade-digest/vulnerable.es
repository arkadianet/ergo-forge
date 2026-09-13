{
  val next = OUTPUTS(0)
  val upgrade = blake2b256(next.propositionBytes) == SELF.R4[Coll[Byte]].get
  val continue = next.propositionBytes == SELF.propositionBytes && next.value >= SELF.value
  sigmaProp(upgrade || continue)
}
