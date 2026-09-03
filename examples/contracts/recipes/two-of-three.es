/**
 * Any two of three people must sign to spend. A shared account, a small
 * treasury, or a backup key arrangement.
 * @param first the first signer's address
 * @param second the second signer's address
 * @param third the third signer's address
 */
@contract def twoOfThree(first: SigmaProp, second: SigmaProp, third: SigmaProp) =
  atLeast(2, Coll(first, second, third))
