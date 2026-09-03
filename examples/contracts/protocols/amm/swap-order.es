// A swap order: ERG locked here may be exchanged against the pool named by
// its NFT, as long as the trader receives at least the minimum amount of
// the token. The pool script keeps the product of the reserves; this
// script keeps the trader from a bad fill. The trader can cancel any time.
//
// $trader: SigmaProp    -- who placed the order and receives the tokens
// $poolNft: Coll[Byte]  -- the pool this order may trade against
// $tokenY: Coll[Byte]   -- the token being bought
// $minOutput: Long      -- the least amount of tokenY the trader accepts
{
  val poolIn = INPUTS(0)
  val executed = INPUTS.size > 1 && OUTPUTS.size > 1 &&
    poolIn.tokens.size > 0 && poolIn.tokens(0)._1 == $poolNft &&
    OUTPUTS(1).propositionBytes == $trader.propBytes &&
    OUTPUTS(1).tokens.size > 0 && OUTPUTS(1).tokens(0)._1 == $tokenY &&
    OUTPUTS(1).tokens(0)._2 >= $minOutput
  $trader || sigmaProp(executed)
}
