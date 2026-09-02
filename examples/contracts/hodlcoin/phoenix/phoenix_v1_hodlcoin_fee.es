{

    // ===== Contract Information ===== //
    // Name: Phoenix HodlCoin Fee
    // Description: Contract guarding the fee box for the HodlCoin protocol.
    // Version: 1.0.0
    // Author: Luca D'Angelo (ldgaetano@protonmail.com), MGPai

    // ===== Box Contents ===== //
    // Tokens
    // None
    // Registers
    // None

    // ===== Relevant Transactions ===== //
    // 1. Fee Distribution Tx
    // Inputs: PhoenixFee1, ... , PhoenixFeeM
    // DataInputs: None
    // Outputs: Bruno, Pulsarz, Phoenix, Kushti, Kras, MinerFee
    // Context Variables: None

    // ===== Compile Time Constants ($) ===== //
    // $minerFee: Long

    // ===== Context Variables (@) ===== //
    // None

    // ===== Relevant Variables ===== //
    val minerFeeErgoTreeBytesHash: Coll[Byte] = fromBase16("e540cceffd3b8dd0f401193576cc413467039695969427df94454193dddfb375")

    val feeDenom: Long   = 100L
    val brunoNum: Long   = 25L
    val pulsarzNum: Long = 25L
    val phoenixNum: Long = 25L
    val kushtiNum: Long  = 15L
    val krasNum: Long    = 10L

    val brunoAddress: SigmaProp   = PK("9gnBtmSRBMaNTkLQUABoAqmU2wzn27hgqVvezAC9SU1VqFKZCp8")
    val pulsarzAddress: SigmaProp = PK("9hHondX3uZMY2wQsXuCGjbgZUqunQyZCNNuwGu6rL7AJC8dhRGa")
    val phoenixAddress: SigmaProp = PK("9iPs1ujGj2eKXVg82aGyAtUtQZQWxFaki48KFixoaNmUAoTY6wV")
    val kushtiAddress: SigmaProp  = PK("9iE2MadGSrn1ivHmRZJWRxzHffuAk6bPmEv6uJmPHuadBY8td5u")
    val krasAddress: SigmaProp    = PK("9g8QaPWTnx71tDdqgzdqZVdNg1cKgEHhAhjCvFqV7ggxRVwycCg")

    // ===== Fee Distribution Tx ===== //
    val validFeeDistributionTx: Boolean = {

    // Outputs
    val brunoBoxOUT: Box    = OUTPUTS(0)
    val pulsarzBoxOUT: Box  = OUTPUTS(1)
    val phoenixBoxOUT: Box  = OUTPUTS(2)
    val kushtiBoxOUT: Box   = OUTPUTS(3)
    val krasBoxOUT: Box     = OUTPUTS(4)
    val minerFeeBoxOUT: Box = OUTPUTS(5)

    val outputAmount: Long = OUTPUTS.map({ (output: Box) => output.value }).fold(0L, { (acc: Long, curr: Long) => acc + curr })
    val devAmount: Long = outputAmount - minerFeeBoxOUT.value // In case the miner fee increases in the future.

    val validMinAmount: Boolean = (outputAmount >= 5000000L) // This prevents dust transactions

    val validDevBoxes: Boolean = {

    val brunoAmount: Long   = (brunoNum * devAmount) / feeDenom
    val pulsarzAmount: Long = (pulsarzNum * devAmount) / feeDenom
    val phoenixAmount: Long = (phoenixNum * devAmount) / feeDenom
    val kushtiAmount: Long  = (kushtiNum * devAmount) / feeDenom
    val krasAmount: Long    = (krasNum * devAmount) / feeDenom

    val validBruno: Boolean   = (brunoBoxOUT.value == brunoAmount) && (brunoBoxOUT.propositionBytes == brunoAddress.propBytes)
    val validPulsarz: Boolean = (pulsarzBoxOUT.value == pulsarzAmount) && (pulsarzBoxOUT.propositionBytes == pulsarzAddress.propBytes)
    val validPhoenix: Boolean = (phoenixBoxOUT.value == phoenixAmount) && (phoenixBoxOUT.propositionBytes == phoenixAddress.propBytes)
    val validKushti: Boolean  = (kushtiBoxOUT.value == kushtiAmount) && (kushtiBoxOUT.propositionBytes == kushtiAddress.propBytes)
    val validKras: Boolean    = (krasBoxOUT.value == krasAmount) && (krasBoxOUT.propositionBytes == krasAddress.propBytes)

    allOf(Coll(
        validBruno,
        validPulsarz,
        validPhoenix,
        validKushti,
        validKras
    ))

}

    val validMinerFee: Boolean = {

    allOf(Coll(
        (minerFeeBoxOUT.value >= $minerFee), // In case the miner fee increases in the future
            (blake2b256(minerFeeBoxOUT.propositionBytes) == minerFeeErgoTreeBytesHash)
    ))

}

    val validOutputSize: Boolean = (OUTPUTS.size == 6)

    allOf(Coll(
        validMinAmount,
        validDevBoxes,
        validMinerFee,
        validOutputSize
    ))

}

    sigmaProp(validFeeDistributionTx) && atLeast(1, Coll(brunoAddress, pulsarzAddress, phoenixAddress, kushtiAddress, krasAddress)) // Done so we are incentivized to not spam the miner fee.

}