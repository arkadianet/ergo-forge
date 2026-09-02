{

    // ===== Contract Information ===== //
    // Name: Phoenix HodlToken Bank
    // Description: Contract for the bank box of the hodlToken protocol.
    // Version: 1.0.0
    // Author: Luca D'Angelo (ldgaetano@protonmail.com), MGPai, Kushti

    // ===== Box Contents ===== //
    // Tokens
    // 1. (BankSingletonId, 1)
    // 2. (HodlTokenId, HodlTokenAmount)
    // 3. (BaseTokenId, BaseTokenAmount)
    // Registers
    // R4: Long             TotalTokenSupply
    // R5: Long             PrecisionFactor
    // R6: Long             MinBankValue
    // R7: Long             BankFeeNum
    // R8: Long             DevFeeNum

    // ===== Relevant Transactions ===== //
    // 1. Mint Tx
    // Inputs: Bank, Proxy
    // Data Inputs: None
    // Outputs: Bank, BuyerPK, MinerFee, TxOperatorFee
    // Context Variables: None
    // 2. Burn Tx
    // Inputs: Bank, Proxy
    // Data Inputs: None
    // Outputs: Bank, BuyerPK, PhoenixFee, MinerFee, TxOperatorFee
    // Context Variables: None
    // 3. Reserve Deposit Tx
    // Inputs: Bank, Proxy
    // Data Input: None
    // Outputs: Bank, MinerFee, TxOperatorFee
    // Context Variables: None

    // ===== Compile Time Constants ($) ===== //
    // $phoenixFeeContractBytes: Coll[Byte]

    // ===== Context Variables (_) ===== //
    // None

    // ===== User Defined Methods ===== //
    // divUp: (BigInt, BigInt) => BigInt

    // Integer division, rounded up.
    def divUp(operands: (BigInt, BigInt)): BigInt = {

        val a: BigInt = operands._1 // Dividend
        val b: BigInt = operands._2 // Divisor

        if (b == 0.toBigInt) {
            -1.toBigInt
        } else {
            (a + (b-1.toBigInt)) / b
        }

    }

    // ===== Relevant Variables ===== //
    val totalTokenSupply: Long      = SELF.R4[Long].get
    val precisionFactor: Long       = SELF.R5[Long].get // If the token does not have decimals, i.e. decimals = 0 when minted, then the precision factor should be set to 1.
    val minBankValue: Long          = SELF.R6[Long].get
    val devFeeNum: Long             = SELF.R7[Long].get
    val bankFeeNum: Long            = SELF.R8[Long].get
    val feeDenom: BigInt            = 1000.toBigInt

    // Bank Input
    val hodlTokensIn: Long       = SELF.tokens(1)._2                // hodlToken token amount in the bank box.
    val reserveIn: Long          = SELF.tokens(2)._2                // Amount of base token in the bank box
    val hodlTokensCircIn: Long   = totalTokenSupply - hodlTokensIn  // hodlToken in circulation since this value represents what is not inside the box, this must not ever be 0.

    // Bank Output
    val bankBoxOUT: Box      = OUTPUTS(0)
    val reserveOut: Long     = bankBoxOUT.tokens(2)._2
    val hodlTokensOut: Long  = bankBoxOUT.tokens(1)._2

    // Bank Info
    val hodlTokensCircDelta: Long   = hodlTokensIn - hodlTokensOut                                  // When minting hodlToken, this is the amount of coins the buyer gets.
    val price: BigInt               = (reserveIn.toBigInt * precisionFactor) / hodlTokensCircIn
    val isMintTx: Boolean           = (hodlTokensCircDelta > 0L)                                    // hodlToken supply increases + baseToken reserve increases
    val isBurnTx: Boolean           = (hodlTokensCircDelta < 0L)                                    // hodlToken supply decreases + baseToken reserve decreases
    val isDepositTx: Boolean        = (hodlTokensCircDelta == 0L)                                   // baseToken reserve increases

    val validBankRecreation: Boolean = {

        val validValue: Boolean = (bankBoxOUT.value == SELF.value) // ERG value in bank box should not change for any reason, since HoldToken version of HodlCoin has nothing to do with ERG.

        val validContract: Boolean = (bankBoxOUT.propositionBytes == SELF.propositionBytes)

        val validTokens: Boolean = {

            val validBankSingleton: Boolean = (bankBoxOUT.tokens(0) == SELF.tokens(0))          // Singleton token amount never changes
            val validHodlTokenId: Boolean = (bankBoxOUT.tokens(1)._1 == SELF.tokens(1)._1)
            val validHodlTokenAmount: Boolean = (hodlTokensOut >= 1L)                           // HodlToken token amount can change, but there must be 1 hodlerg inside the bank always
            val validBaseTokenId: Boolean = (bankBoxOUT.tokens(2)._1 == SELF.tokens(2)._1)
            val validBaseTokenMinBankValue: Boolean = (reserveOut >= minBankValue)              // The bank must have a minimum value of the base token.

            allOf(Coll(
                validBankSingleton,
                validHodlTokenId,
                validHodlTokenAmount,
                validBaseTokenId,
                validBaseTokenMinBankValue
            ))

        }

        val validRegisters: Boolean = {

            allOf(Coll(
                (bankBoxOUT.R4[Long].get == SELF.R4[Long].get),
                (bankBoxOUT.R5[Long].get == SELF.R5[Long].get),
                (bankBoxOUT.R6[Long].get == SELF.R6[Long].get),
                (bankBoxOUT.R7[Long].get == SELF.R7[Long].get),
                (bankBoxOUT.R8[Long].get == SELF.R8[Long].get)
            ))

        }

        allOf(Coll(
            validValue,
            validContract,
            validTokens,
            validRegisters
        ))

    }

    if (isMintTx) {

        // ===== Mint Tx ===== //
        val validMintTx: Boolean = {

            val expectedAmountDeposited: Long = (hodlTokensCircDelta * price) / precisionFactor // Price of hodlCoin in nanoERG.

            val validTokenDeposit: Boolean = (reserveOut >= reserveIn + expectedAmountDeposited)

            allOf(Coll(
                validBankRecreation,
                validTokenDeposit
            ))

        }

        sigmaProp(validMintTx)

    } else if (isBurnTx) {

        // ===== Burn Tx ===== //
        val validBurnTx: Boolean = {

            val hodlTokensBurned: Long = hodlTokensOut - hodlTokensIn
            val expectedAmountBeforeFees: Long = (hodlTokensBurned * price) / precisionFactor // X: Here we convert the amount of hodlTokens burned into the amount of base tokens released from the bank.

            val dividend_1: BigInt = (expectedAmountBeforeFees.toBigInt * (bankFeeNum.toBigInt + devFeeNum.toBigInt)) // Here we want to determine the amount allocated to the bank and to the developers.
            val divisor_1: BigInt = feeDenom // This is never zero.

            val bankFeeAndDevFeeAmount: BigInt = divUp((dividend_1, divisor_1)) // Y: Here we use the divUp method to perform integer division in order to compute the combined bank fee and developer fee without integer division errors.

            val dividend_2: BigInt = (bankFeeAndDevFeeAmount.toBigInt * devFeeNum.toBigInt) // Here we want to determine the allocation of the developers from the total bank fee and developer fee.
            val divisor_2: BigInt = (bankFeeNum.toBigInt + devFeeNum.toBigInt) // This is never zero, devFeeNum can be zero but bankFeeNum cannot.

            val devFeeAmount: BigInt = divUp((dividend_2, divisor_2)) // Z: Here we use the divUp method to perform integer division in order to compute the developer fee allocation without integer division errors.
            val bankFeeAmount: BigInt = bankFeeAndDevFeeAmount - devFeeAmount // Y - Z: Using the the combined bank fee and developer fee and the isolated developer fee, we compute the remaining amount which is the bank fee allocation.

            // Here we adjust the developer fee and bank fee amounts so that the bank fee is never 0, we do this to preserve the game mechanics of the protocol, i.e. that the price always increases.
            val devFeeAmountAdjusted: BigInt = if (bankFeeAmount == 0.toBigInt) 0.toBigInt else devFeeAmount
            val bankFeeAmountAdjusted: BigInt = if (bankFeeAmount == 0.toBigInt) devFeeAmount else bankFeeAmount

            val expectedUserAmount: BigInt = expectedAmountBeforeFees - bankFeeAndDevFeeAmount // X - Y: The buyer never gets the bankFeeAmount since it remains in the bank box.

            val validBankWithdraw: Boolean = (reserveOut.toBigInt == reserveIn.toBigInt - expectedAmountBeforeFees + bankFeeAmountAdjusted)

            val validPhoenixFee: Boolean = {

                if (devFeeAmountAdjusted != 0.toBigInt) {

                    val phoenixFeeBoxOUT: Box = OUTPUTS(2)

                    allOf(Coll(
                        (phoenixFeeBoxOUT.tokens(0)._1 == SELF.tokens(2)._1),
                        (phoenixFeeBoxOUT.tokens(0)._2 == devFeeAmountAdjusted),
                        (phoenixFeeBoxOUT.propositionBytes == $phoenixFeeContractBytes)
                    ))

                } else {
                    true
                }

            }

            allOf(Coll(
                validBankRecreation,
                validBankWithdraw,
                validPhoenixFee
            ))

        }

        sigmaProp(validBurnTx)

    } else if (isDepositTx) {

        // ===== Reseve Deposit Tx ===== //
        val validReserveDepositTx: Boolean = {

           val validReserveIncrease: Boolean = (reserveOut > reserveIn) // The bank baseToken reserve must increase, nothing else happens.

           allOf(Coll(
            validBankRecreation,
            validReserveIncrease
           ))

        }

        sigmaProp(validReserveDepositTx)

    } else {
        sigmaProp(false)
    }

}