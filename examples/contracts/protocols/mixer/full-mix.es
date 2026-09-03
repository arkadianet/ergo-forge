// A full-mix box (the shape of ZeroJoin / ErgoMixer). Two such boxes are
// created when a half-mix box is spent: (c1, c2) = (g^y, u^y) and its
// mirror (u^y, g^y), where u = g^x is the half-mix owner's key and y the
// spender's secret. Each box can be spent by exactly one of the two: the
// one whose c2 is g^y by proving its discrete log (the spender), the one
// whose c2 is c1^x by proving the Diffie-Hellman tuple (the owner). An
// outsider cannot tell which is which.
//
// $u: GroupElement  -- the half-mix owner's key g^x
{
  val c1 = SELF.R4[GroupElement].get
  val c2 = SELF.R5[GroupElement].get
  proveDlog(c2) || proveDHTuple(groupGenerator, c1, $u, c2)
}
