{
  // ErgoScript LSP - Hover Feature Demo
  //
  // Hover over any symbol to see rich documentation with:
  // - Type signatures
  // - Detailed descriptions
  // - Usage examples
  // - Related symbols
  //
  // Try hovering over the symbols marked with ↓ below:

  // 1. Keywords
  // Hover over "val" to see documentation for value declarations
  val deadline = 100000
  //↑

  // 2. Global Constants
  // Hover over "SELF" to see documentation about the current box
  val boxValue = SELF.value
  //             ↑

  // Hover over "HEIGHT" to see blockchain height documentation
  val currentHeight = HEIGHT
  //                  ↑

  // 3. Box Properties
  // Hover over "value" to see ERG amount documentation
  val amount = OUTPUTS(0).value
  //                      ↑

  // Hover over "R4" to see register documentation with type info
  val customData = SELF.R4[Int].get
  //                    ↑

  // Hover over "tokens" to see token collection documentation
  val boxTokens = SELF.tokens
  //                   ↑

  // 4. Option Methods
  // Hover over "get" to see extraction method docs (with warnings!)
  val extractedValue = SELF.R5[Long].get
  //                                 ↑

  // Hover over "getOrElse" to see safe extraction docs
  val safeValue = SELF.R6[Int].getOrElse(0)
  //                            ↑

  // Hover over "isDefined" to see presence check docs
  val hasData = SELF.R7[Coll[Byte]].isDefined
  //                                 ↑

  // 5. Functions
  // Hover over "sigmaProp" to see comprehensive function docs
  sigmaProp(HEIGHT > deadline)
  //↑

  // Hover over "blake2b256" to see hash function docs
  val hash = blake2b256(SELF.propositionBytes)
  //         ↑

  // Hover over "atLeast" to see threshold signature docs
  // atLeast(2, Coll(pk1, pk2, pk3))
  //    ↑

  // 6. Collection Methods
  // Hover over "map" to see transformation method docs
  // val values = INPUTS.map((box) => box.value)
  //                     ↑

  // Hover over "filter" to see filtering method docs
  // val largeBoxes = OUTPUTS.filter((box) => box.value > 1000)
  //                          ↑

  // Hover over "exists" to see existence check docs
  // val hasToken = SELF.tokens.exists((t) => t._1 == tokenId)
  //                            ↑

  // 7. Types
  // Hover over "Int" to see integer type docs
  val counter: Int = 42
  //           ↑

  // Hover over "Long" to see long integer type docs
  val nanoErgs: Long = 1000000000L
  //            ↑

  // Hover over "Box" to see UTXO box type docs
  val myBox: Box = OUTPUTS(0)
  //         ↑

  // Hover over "Coll" to see collection type docs
  val bytes: Coll[Byte] = blake2b256(hash)
  //         ↑

  // 8. Complex Expressions
  // You can hover over any symbol in complex expressions
  val condition = HEIGHT > deadline && SELF.value >= 1000000
  //              ↑      ↑           ↑    ↑     ↑
  //    All these symbols have hover documentation!

  // 9. Conditional Expressions
  // Hover over "if" to see conditional expression docs
  val result = if (HEIGHT > 100) true else false
  //           ↑

  // Final contract expression
  sigmaProp(condition)
}

// Pro Tips:
// 1. Hover shows the full identifier range (highlighted)
// 2. Related symbols help you discover related functionality
// 3. Examples show real-world usage patterns
// 4. Category tags help you understand the symbol type
// 5. Type signatures use standard ErgoScript syntax
