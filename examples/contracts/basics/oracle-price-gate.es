// Spend only when an oracle box (a data input carrying $oracleNFT) reports
// a price above the floor. Supply the oracle box as a data input in the
// spend hunt to see it pass.
// $oracleNFT: Coll[Byte]
// $floor: Long
{
  val oracle = CONTEXT.dataInputs(0)
  val isOracle = oracle.tokens(0)._1 == $oracleNFT
  val price = oracle.R4[Long].get
  sigmaProp(isOracle && price > $floor)
}
