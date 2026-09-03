/**
 * A savings box you can only draw from a little at a time: at most a fixed
 * amount per period, the rest staying locked. A guard against spending it
 * all at once. Closing the box entirely needs a second, backup key.
 * @param owner Who may withdraw? — Your everyday address.
 * @param backup Who may empty the box together with the owner? — A backup key kept safely.
 * @param cap How much may be taken per period, in nanoERG — 1 ERG = 1,000,000,000 nanoERG.
 * @param periodBlocks How many blocks between withdrawals? — About 720 per day.
 */
@contract def savingsCap(owner: SigmaProp, backup: SigmaProp, cap: Long, periodBlocks: Int) = {
  val lastDraw = SELF.R4[Int].getOrElse(0)
  val due = HEIGHT >= lastDraw + periodBlocks
  val rest = OUTPUTS(0)
  val smallDraw = rest.propositionBytes == SELF.propositionBytes &&
                  rest.value >= SELF.value - cap &&
                  rest.R4[Int].get == HEIGHT
  (owner && sigmaProp(due && smallDraw)) || (owner && backup)
}
