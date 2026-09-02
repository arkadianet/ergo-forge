/*
 * Sample ErgoScript contract template
 * @param minHeight Minimum blockchain height
 */
@contract def heightLock(minHeight: Int = 100) = {
  HEIGHT > minHeight
}
