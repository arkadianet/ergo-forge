/*
 * Multi-Signature Contract - EIP-5 Template Example
 *
 * This contract requires at least N signatures from a collection of public keys.
 * Demonstrates parameterized contract with complex types.
 *
 * @param threshold Number of signatures required
 * @param publicKeys Collection of public keys that can sign
 */
@contract def multiSig(
  threshold: Int = 2,
  publicKeys: Coll[SigmaProp] = Coll(
    PK("9f5ZKbECVTm25JTRQHDHGM5ehC8tUw5g1fCBQ4aaE792rWBFrjK"),
    PK("9fRusAarL1KkrWQVsxSRVYnvWmjvHJ4VzxHVDMXzeZLQnCKjx9C"),
    PK("9fPiW45mZwoTxSwTLLXaZcdekqi72emebENmScyTGsjryzrntUe")
  )
) = {
  atLeast(threshold, publicKeys)
}
