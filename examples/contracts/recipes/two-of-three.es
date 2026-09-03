/**
 * A shared account: any two of three people must agree to spend. Good for a
 * small treasury, a couple with a backup key, or a company.
 * @param first First person — An Ergo address.
 * @param second Second person — An Ergo address.
 * @param third Third person — An Ergo address.
 */
@contract def twoOfThree(first: SigmaProp, second: SigmaProp, third: SigmaProp) =
  atLeast(2, Coll(first, second, third))
