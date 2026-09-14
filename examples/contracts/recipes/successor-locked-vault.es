/**
 * Keep a vault's identity and state together whenever it moves.
 * Walkthrough: put the vault at INPUTS(0) and its successor at OUTPUTS(0).
 * Carry the same script, singleton NFT, all tokens, at least the current
 * value and the chosen value floor, and both named registers: R4[Long]
 * (accounting units) and R5[Coll[Byte]] (state tag). Exactly one output is
 * allowed in this teaching shape. Try lowering its value, changing R4,
 * or dropping R5: each must fail even when the script and NFT stay intact.
 * Anyone may perform this preservation step; there is no withdrawal path.
 * The suites are synthetic reductions, not complete signed transactions.
 * @param vaultNFT Which vault? — Its singleton NFT id, held in token slot zero.
 * @param floorValue Minimum vault value? — The successor must also retain at least the current value.
 */
@contract def successorLockedVault(vaultNFT: Coll[Byte], floorValue: Long) = {
  if (OUTPUTS.size == 1 && SELF.tokens.size > 0) {
    val next = OUTPUTS(0)
    val units = SELF.R4[Long].getOrElse(-1L)
    val tag = SELF.R5[Coll[Byte]].getOrElse(Coll[Byte]())
    sigmaProp(SELF.tokens(0)._1 == vaultNFT && SELF.tokens(0)._2 == 1L &&
      next.propositionBytes == SELF.propositionBytes && next.tokens == SELF.tokens &&
      next.value >= SELF.value && next.value >= floorValue &&
      SELF.R4[Long].isDefined && SELF.R5[Coll[Byte]].isDefined && units >= 0L &&
      next.R4[Long].isDefined && next.R4[Long].getOrElse(-1L) == units &&
      next.R5[Coll[Byte]].isDefined && next.R5[Coll[Byte]].getOrElse(Coll[Byte]()) == tag)
  } else sigmaProp(false)
}
