/*
 * Simple Height Lock Contract - EIP-5 Template Example
 *
 * This contract locks funds until a specific block height is reached.
 * It demonstrates the EIP-5 contract template syntax.
 *
 * @param minHeight The minimum block height required to spend the funds
 */
@contract def heightLock(minHeight: Int = 100) = {
  HEIGHT > minHeight
}
