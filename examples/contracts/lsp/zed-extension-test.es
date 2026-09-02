/*
 * This is a test ErgoScript file to demonstrate Zed extension capabilities
 */
@contract def myTestContract(delay: Int = 100) = {
  // Simple ErgoScript contract example
  val deadline = HEIGHT + delay
  val pk = PK("9hxa8WUf2RRCGqVJD73FM8fPc4oj3j7cUtoauuT3zUxVKPqvT7X")
  /*
   * This contract allows spending after a certain height
   * with proper signature verification
   */
  val condition = sigmaProp(HEIGHT > deadline)
  val test = SELF.bytesWithoutRef.size

  val conditions = Coll(100, 1000).filter({(i: Int) => i > 500})

  val tupl = (100L, 1000)

  allOf(Coll(condition, pk))
}
