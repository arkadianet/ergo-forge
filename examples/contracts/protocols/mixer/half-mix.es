// A half-mix box (the shape of ZeroJoin / ErgoMixer): funds waiting for a
// partner. R4 holds the owner's key u = g^x. The owner may take the funds
// back. Anyone else may spend it only into exactly two full-mix boxes of
// the same value, (c1, c2) and (c2, c1), and must PROVE that c1 = g^y and
// c2 = u^y for a y they know (a Diffie-Hellman tuple) — which is what
// makes one box the owner's and the other the spender's.
//
// $fullMix: Coll[Byte]  -- the full-mix script's bytes (compiled with this u)
{
  val u = SELF.R4[GroupElement].get
  val owner = proveDlog(u)
  if (OUTPUTS.size != 2) owner else {
    val o0 = OUTPUTS(0)
    val o1 = OUTPUTS(1)
    val ok = o0.R4[GroupElement].isDefined && o0.R5[GroupElement].isDefined &&
      o1.R4[GroupElement].isDefined && o1.R5[GroupElement].isDefined
    if (!ok) owner else {
      val c1 = o0.R4[GroupElement].get
      val c2 = o0.R5[GroupElement].get
      val wellFormed =
        o0.propositionBytes == $fullMix && o1.propositionBytes == $fullMix &&
        o1.R4[GroupElement].get == c2 && o1.R5[GroupElement].get == c1 &&
        o0.value == SELF.value && o1.value == SELF.value && c1 != c2
      owner || (sigmaProp(wellFormed) && proveDHTuple(groupGenerator, u, c1, c2))
    }
  }
}
