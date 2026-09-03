/**
 * Lock funds until a block height, then only the owner can spend.
 * Useful for savings you do not want to touch before a date.
 * @param owner the address that may spend once the lock has passed
 * @param unlockHeight the block height at which spending becomes possible
 */
@contract def timeLock(owner: SigmaProp, unlockHeight: Int) =
  owner && sigmaProp(HEIGHT >= unlockHeight)
