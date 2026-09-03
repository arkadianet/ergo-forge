/**
 * Spend only while an oracle reports a price above a floor. The spending
 * transaction must include the oracle's box as a data input.
 * @param owner Who may spend? — An Ergo address.
 * @param oracleNFT Which oracle? — The token id that identifies the oracle box.
 * @param floor Minimum price — In the oracle's own units.
 */
@contract def priceGate(owner: SigmaProp, oracleNFT: Coll[Byte], floor: Long) = {
  val oracle = CONTEXT.dataInputs(0)
  val isOracle = oracle.tokens(0)._1 == oracleNFT
  val price = oracle.R4[Long].get
  owner && sigmaProp(isOracle && price >= floor)
}
