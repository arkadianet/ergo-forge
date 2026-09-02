/*
 * Payment Channel Contract - EIP-5 Template Example
 *
 * This contract implements a simple payment channel where:
 * - Funds can be spent by the recipient if they have a signature from the sender
 * - After a timeout, the sender can reclaim their funds
 *
 * @param senderPK Public key of the sender who can reclaim after timeout
 * @param recipientPK Public key of the recipient who can spend with sender's signature
 * @param timeout Block height after which sender can reclaim funds
 */
@contract def paymentChannel(
  senderPK: SigmaProp = PK("9f5ZKbECVTm25JTRQHDHGM5ehC8tUw5g1fCBQ4aaE792rWBFrjK"),
  recipientPK: SigmaProp = PK("9fRusAarL1KkrWQVsxSRVYnvWmjvHJ4VzxHVDMXzeZLQnCKjx9C"),
  timeout: Int = 1000
) = {
  val recipientPath = recipientPK && senderPK
  val timeoutPath = senderPK && (HEIGHT > timeout)
  recipientPath || timeoutPath
}
