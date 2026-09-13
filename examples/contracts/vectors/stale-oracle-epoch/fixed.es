{
  val oracle = CONTEXT.dataInputs(0)
  sigmaProp(oracle.tokens(0)._1 == fromBase16("0101010101010101010101010101010101010101010101010101010101010101") &&
    oracle.R4[Long].get >= 100L && oracle.R5[Int].get == HEIGHT / 10 &&
    oracle.creationInfo._1 >= HEIGHT - 5 && oracle.creationInfo._1 <= HEIGHT)
}
