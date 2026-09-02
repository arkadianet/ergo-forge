{

    // ===== Contract Information ===== //
    // Name: Phoenix HodlCoin Proxy
    // Description: Contract guarding the proxy box for the HodlCoin protocol.
    // Version: 1.0.0
    // Author: Luca D'Angelo (ldgaetano@protonmail.com), MGPai

    // ===== Box Contents ===== //
    // Tokens
    // 1. (HodlCoinTokenId, HodlCoinTokenAmount) if burning hodlCoin tokens.
    // Registers
    // R4: SigmaProp    BuyerPK
    // R5: Coll[Byte]   BankSingletonTokenId
    // R6: Coll[Byte]   HodlCoinTokenId
    // R7: Long         MinBoxValue
    // R8: Long         MinerFee
    // R9: Long         TxOperatorFee

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
    // 3. Refund Tx
    // Inputs: Proxy
    // Data Inputs: None
    // Outputs: BuyerPK, MinerFee
    // Context Variables: None

    // ===== Compile Time Constants ($) ===== //
    // $minTxOperatorFee: Long

    // ===== Context Variables (@) ===== //
    // None

    // ===== Relevant Variables ===== //
    val buyerPK: SigmaProp                      = SELF.R4[SigmaProp].get
    val bankSingletonTokenId: Coll[Byte]        = SELF.R5[Coll[Byte]].get
    val hodlCoinTokenId: Coll[Byte]             = SELF.R6[Coll[Byte]].get
    val minBoxValue: Long                       = SELF.R7[Long].get
    val minerFee: Long                          = SELF.R8[Long].get
    val txOperatorFee: Long                     = SELF.R9[Long].get
    val minerFeeErgoTreeBytesHash: Coll[Byte]   = fromBase16("e540cceffd3b8dd0f401193576cc413467039695969427df94454193dddfb375")
    val isValidBank: Boolean                    = (INPUTS(0).tokens.size > 1 && INPUTS(0).tokens(0)._1 == bankSingletonTokenId) && (INPUTS(0).tokens(1)._1 == hodlCoinTokenId)

    if (isValidBank) {

        // Bank Input
        val bankBoxIN: Box              = INPUTS(0)
        val reserveIn: Long             = bankBoxIN.value
        val hodlCoinsIn: Long           = bankBoxIN.tokens(1)._2
        val totalTokenSupply: Long      = bankBoxIN.R4[Long].get
        val precisionFactor: Long       = bankBoxIN.R5[Long].get
        val bankFeeNum: Long            = bankBoxIN.R7[Long].get
        val devFeeNum: Long             = bankBoxIN.R8[Long].get
        val feeDenom: Long              = 1000L
        val hodlCoinsCircIn: Long       = totalTokenSupply - hodlCoinsIn

        // Bank Output
        val bankBoxOUT: Box     = OUTPUTS(0)
        val reserveOut: Long    = bankBoxOUT.value
        val hodlCoinsOut: Long  = bankBoxOUT.tokens(1)._2

        // Bank Info
        val hodlCoinsCircDelta: Long    = hodlCoinsIn - hodlCoinsOut
        val price: Long                 = (reserveIn.toBigInt * precisionFactor) / hodlCoinsCircIn
        val isMintTx: Boolean           = (hodlCoinsCircDelta > 0L)

        // Outputs
        val buyerPKBoxOUT: Box = OUTPUTS(1)

        if (isMintTx) {

            // ===== Mint Tx ===== //
            val validMintTx: Boolean = {

                // Outputs
                val minerFeeBoxOUT: Box = OUTPUTS(2)
            val txOperatorFeeBoxOUT: Box = OUTPUTS(3)

            val expectedAmountDeposited: Long = (hodlCoinsCircDelta * price) / precisionFactor

            val validProxyValue: Boolean = (SELF.value - minBoxValue - minerFee - txOperatorFee >= expectedAmountDeposited)

            val validBuyerBoxOUT: Boolean = {

                val validValue: Boolean = (buyerPKBoxOUT.value == minBoxValue)
            val validContract: Boolean = (buyerPKBoxOUT.propositionBytes == buyerPK.propBytes)
            val validHodlCoinTransfer: Boolean = (buyerPKBoxOUT.tokens(0) == (bankBoxOUT.tokens(1)._1, hodlCoinsCircDelta))

            allOf(Coll(
                validValue,
                validContract,
                validHodlCoinTransfer
            ))

        }

            val validMinerFee: Boolean = {

                allOf(Coll(
                    (minerFeeBoxOUT.value == minerFee),
                        (blake2b256(minerFeeBoxOUT.propositionBytes) == minerFeeErgoTreeBytesHash)
                ))

        }

            val validTxOperatorFee: Boolean = {

                allOf(Coll(
                    (txOperatorFee >= $minTxOperatorFee),
                        (txOperatorFeeBoxOUT.value == txOperatorFee)
                ))

        }

            val validOutputSize: Boolean = (OUTPUTS.size == 4)

            allOf(Coll(
                validProxyValue,
                validBuyerBoxOUT,
                validMinerFee,
                validTxOperatorFee,
                validOutputSize
            ))

        }

            sigmaProp(validMintTx)

        } else {

            // ===== Burn Tx ===== //
            val validBurnTx: Boolean = {

                // Outputs
                val phoenixFeeBoxOUT: Box = OUTPUTS(2)
            val minerFeeBoxOUT: Box = OUTPUTS(3)
            val txOperatorFeeBoxOUT: Box = OUTPUTS(4)

            val hodlCoinsBurned: Long = hodlCoinsOut - hodlCoinsIn
            val expectedAmountBeforeFees: Long = (hodlCoinsBurned * price) / precisionFactor
            val bankFeeAmount: Long = (expectedAmountBeforeFees * bankFeeNum) / feeDenom
            val devFeeAmount: Long = (expectedAmountBeforeFees * devFeeNum) / feeDenom
            val expectedAmountWithdrawn: Long = expectedAmountBeforeFees - bankFeeAmount - devFeeAmount

            val validBurn: Boolean = (bankBoxOUT.tokens(1)._2 == bankBoxIN.tokens(1)._2 + SELF.tokens(0)._2)

            val validBuyerBoxOUT: Boolean = {

                val validERGTransfer: Boolean = (buyerPKBoxOUT.value == expectedAmountWithdrawn)
            val validContract: Boolean = (buyerPKBoxOUT.propositionBytes == buyerPK.propBytes)

            allOf(Coll(
                validERGTransfer,
                validContract
            ))

        }

            val validMinerFee: Boolean = {

                allOf(Coll(
                    (minerFeeBoxOUT.value == minerFee),
                        (blake2b256(minerFeeBoxOUT.propositionBytes) == minerFeeErgoTreeBytesHash)
                ))

        }

            val validTxOperatorFee: Boolean = {

                allOf(Coll(
                    (txOperatorFee >= $minTxOperatorFee),
                        (txOperatorFeeBoxOUT.value == txOperatorFee)
                ))

        }

            val validOutputSize: Boolean = (OUTPUTS.size == 5)

            allOf(Coll(
                validBurn,
                validBuyerBoxOUT,
                validMinerFee,
                validTxOperatorFee,
                validOutputSize
            ))

        }

            sigmaProp(validBurnTx)

        }

    } else {

        // ===== Refund Tx ===== //
        val validRefundTx: Boolean = {

            val validBuyerBoxOUT: Boolean = {

            // Ensure that the buyer receives the total value of box.
            val validValueTransfer: Boolean = OUTPUTS.map { (o: Box) =>
            if (o.propositionBytes == buyerPK.propBytes) o.value else 0L
        }.fold(0L, { (a: Long, b: Long) => a + b }) >= SELF.value

        // If the box has tokens in it, all must go to buyer.
        val validTokenTransfer: Boolean = {
        if(SELF.tokens.size > 0){
            OUTPUTS.exists { (o: Box) =>
                (o.tokens == SELF.tokens) && (o.propositionBytes == buyerPK.propBytes)
            }
        } else{
            true
        }
    }

        allOf(Coll(
            validValueTransfer,
            validTokenTransfer
        ))

    }

        validBuyerBoxOUT

    }

        sigmaProp(validRefundTx) && buyerPK

    }

}