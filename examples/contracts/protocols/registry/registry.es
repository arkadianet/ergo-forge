// A name registry kept in an AVL+ tree (the shape of ErgoNames). The
// registry box holds the tree's digest in R4; the names themselves live
// off chain with whoever needs them, and every change comes with a proof
// the tree verifies. To register a name the spender supplies, as context
// variables, the proof (0), the name key (1, 32 bytes — hash the name) and
// the record (2, any bytes), rebuilds the box with the new digest, and
// pays the registrar. A name that already exists cannot be inserted: the
// tree refuses the proof. The registrar may spend freely (upgrades).
//
// $registrar: SigmaProp
// $fee: Long
{
  val tree = SELF.R4[AvlTree].get
  val proof = getVar[Coll[Byte]](0)
  val name = getVar[Coll[Byte]](1)
  val record = getVar[Coll[Byte]](2)
  val registered = OUTPUTS.size > 1 && proof.isDefined && name.isDefined && record.isDefined && {
    val successor = OUTPUTS(0)
    val next = tree.insert(Coll((name.get, record.get)), proof.get)
    successor.propositionBytes == SELF.propositionBytes &&
    successor.tokens == SELF.tokens &&
    successor.value >= SELF.value &&
    next.isDefined &&
    successor.R4[AvlTree].isDefined &&
    successor.R4[AvlTree].get.digest == next.get.digest &&
    OUTPUTS(1).propositionBytes == $registrar.propBytes &&
    OUTPUTS(1).value >= $fee
  }
  $registrar || sigmaProp(registered)
}
