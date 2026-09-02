// A register read with no isDefined guard: if R4 is empty the script throws
// and the box is unspendable through this path. The audit flags it.
sigmaProp(SELF.R4[Int].get > 5)
