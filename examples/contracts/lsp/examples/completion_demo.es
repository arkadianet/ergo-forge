{
  // Example ErgoScript contract demonstrating completion features
  // Try triggering completion at the positions marked with ^

  // 1. Type declarations - try typing "val " and you'll get completion
  val deadline = SELF.R4[Int].get
  //              ^^^^  - SELF autocompletes
  //                   ^   - After dot, R4-R9 are suggested
  //                           ^^^  - Type names are suggested
  //                                  ^^^  - Option methods: get, getOrElse, isDefined

  // 2. Global constants - try typing "HEI" and HEIGHT will be suggested
  val currentHeight = HEIGHT
  //                  ^^^^^^  - Global constants autocomplete

  // 3. Functions - try typing "sigma" and sigmaProp will be suggested
  val condition = HEIGHT > deadline
  sigmaProp(condition)
  // ^^^^^^^^  - Function names autocomplete with signature hints

  // 4. Box access - try typing "OUTPUTS(0)." to see box members
  // OUTPUTS(0).value
  //           ^^^^^ - Box members: value, propositionBytes, tokens, etc.

  // 5. Operators and expressions work naturally
  val amount = OUTPUTS(0).value >= 1000000

  // 6. Complex expressions with chaining
  // val hasValidToken = SELF.tokens.exists(...)
  //                          ^^^^^^  - Box property
  //                                 ^^^^^^  - Collection methods
}
