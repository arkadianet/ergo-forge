{
  val next = OUTPUTS(0)
  sigmaProp(SELF.R4[Int].get > 0 &&
    next.propositionBytes == SELF.propositionBytes && next.value >= SELF.value &&
    next.tokens == SELF.tokens)
}
