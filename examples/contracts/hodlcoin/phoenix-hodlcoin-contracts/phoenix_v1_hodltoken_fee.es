{

    // ===== Contract Information ===== //
    // Name: Phoenix HodlToken Fee
    // Description: Contract guarding the fee box for the hodlToken protocol.
    // Version: 1.0.0
    // Author: Luca D'Angelo (ldgaetano@protonmail.com), MGPai

    // ===== Box Contents ===== //
    // Tokens
    // 1. (baseTokenId, baseTokenAmount)
    // Registers
    // None

    // ===== Relevant Transactions ===== //
    // 1. Fee Distribution Tx
    // Inputs: PhoenixFee1, ... , PhoenixFeeM
    // DataInputs: None
    // Outputs: Bruno, Phoenix, Kushti, MinerFee
    // Context Variables: None

    // ===== Compile Time Constants ($) ===== //
    // $baseTokenId: Coll[Byte]
    // $minerFee: Long
    // $brunoNum: Long
    // $phoenixNum: Long
    // $kushtiNum: Long

    // ===== Context Variables (_) ===== //
    // None

    // ===== Relevant Variables ===== //
    val minerFeeErgoTreeBytesHash: Coll[Byte] = fromBase16("e540cceffd3b8dd0f401193576cc413467039695969427df94454193dddfb375")

    val feeDenom: Long = 100L

    val brunoAddress: SigmaProp   = PK("9gnBtmSRBMaNTkLQUABoAqmU2wzn27hgqVvezAC9SU1VqFKZCp8")
    val phoenixAddress: SigmaProp = PK("9iPs1ujGj2eKXVg82aGyAtUtQZQWxFaki48KFixoaNmUAoTY6wV")
    val kushtiAddress: SigmaProp  = PK("9iE2MadGSrn1ivHmRZJWRxzHffuAk6bPmEv6uJmPHuadBY8td5u")

    // ===== Fee Distribution Tx ===== //
    val validFeeDistributionTx: Boolean = {

        // Outputs
        val brunoBoxOUT: Box    = OUTPUTS(0)
        val phoenixBoxOUT: Box  = OUTPUTS(1)
        val kushtiBoxOUT: Box   = OUTPUTS(2)
        val minerFeeBoxOUT: Box = OUTPUTS(3)

        val devAmount: Long = OUTPUTS.flatMap({ (output: Box) =>

            output.tokens.filter({ (token: (Coll[Byte], Long)) => 
        
                token._1 == $baseTokenId 
        
            }).map({ (token: (Coll[Byte], Long)) => 
        
                token._2 
        
            }) 
        
        }).fold(0L, { (acc: Long, curr: Long) => acc + curr })

        val validDevBoxes: Boolean = {

            val brunoAmount: Long   = ($brunoNum * devAmount) / feeDenom
            val phoenixAmount: Long = ($phoenixNum * devAmount) / feeDenom
            val kushtiAmount: Long  = ($kushtiNum * devAmount) / feeDenom

            val validBruno: Boolean   = (brunoBoxOUT.tokens(0)._1 == $baseTokenId) && (brunoBoxOUT.tokens(0)._2 >= brunoAmount) && (brunoBoxOUT.propositionBytes == brunoAddress.propBytes)
            val validPhoenix: Boolean = (phoenixBoxOUT.tokens(0)._1 == $baseTokenId) && (phoenixBoxOUT.tokens(0)._2 >= phoenixAmount) && (phoenixBoxOUT.propositionBytes == phoenixAddress.propBytes)
            val validKushti: Boolean  = (kushtiBoxOUT.tokens(0)._1 == $baseTokenId) && (kushtiBoxOUT.tokens(0)._2 >= kushtiAmount) && (kushtiBoxOUT.propositionBytes == kushtiAddress.propBytes)

            allOf(Coll(
                validBruno,
                validPhoenix,
                validKushti
            ))

        }

        val validMinerFee: Boolean = {

            allOf(Coll(
                (minerFeeBoxOUT.value >= $minerFee), // In case the miner fee increases in the future
                (blake2b256(minerFeeBoxOUT.propositionBytes) == minerFeeErgoTreeBytesHash),
                (minerFeeBoxOUT.tokens.size == 0)
            ))

        }

        val validOutputSize: Boolean = (OUTPUTS.size == 4)

        allOf(Coll(
            validDevBoxes,
            validMinerFee,
            validOutputSize
        ))

    }

    sigmaProp(validFeeDistributionTx) && atLeast(1, Coll(brunoAddress, phoenixAddress, kushtiAddress)) // Done so we are incentivized to not spam the miner fee.

}
