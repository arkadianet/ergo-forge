/**
 * The owner can spend at any time; a second person can spend only after a
 * height. A simple dead-man's switch: if the owner never moves the funds,
 * the heir can claim them later.
 * @param owner the address that may spend at any time
 * @param heir the address that may spend after the height
 * @param heirHeight the block height from which the heir may spend
 */
@contract def inheritance(owner: SigmaProp, heir: SigmaProp, heirHeight: Int) =
  owner || (heir && sigmaProp(HEIGHT >= heirHeight))
