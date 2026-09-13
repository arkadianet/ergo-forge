{
  val amount = SELF.R4[Long].get
  sigmaProp(amount > 0L && OUTPUTS(0).value >= amount / 3L * 2L)
}
