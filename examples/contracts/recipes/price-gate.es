/**
 * The owner can spend only while an oracle reports a price above a floor.
 * The spending transaction must reference the oracle box as a data input.
 * @param owner the address that may spend
 * @param oracleNFT the token id that identifies the oracle box
 * @param floor the price the oracle must report at least, in the oracle's units
 */
@contract def priceGate(owner: SigmaProp, oracleNFT: Coll[Byte], floor: Long) = {
  val oracle = CONTEXT.dataInputs(0)
  val isOracle = oracle.tokens(0)._1 == oracleNFT
  val price = oracle.R4[Long].get
  owner && sigmaProp(isOracle && price >= floor)
}
