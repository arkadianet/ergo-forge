// Anyone may spend, but only back into the same contract: "movable by
// anyone" in the spend hunt, not stealable.
sigmaProp(OUTPUTS(0).propositionBytes == SELF.propositionBytes &&
          OUTPUTS(0).value >= SELF.value)
