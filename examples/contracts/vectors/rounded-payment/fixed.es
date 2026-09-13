{
  val amount = SELF.R4[Long].get
  sigmaProp(amount > 0L && OUTPUTS(0).value.toBigInt * 3L.toBigInt >= amount.toBigInt * 2L.toBigInt)
}
