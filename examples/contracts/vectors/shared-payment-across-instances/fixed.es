sigmaProp(INPUTS.filter({ (b: Box) => b.propositionBytes == SELF.propositionBytes }).size == 1) &&
sigmaProp(OUTPUTS(0).value >= SELF.value && OUTPUTS(0).propositionBytes == fromBase16("0008cd0279be667ef9dcbbac55a06295ce870b07029bfcdb2dce28d959f2815b16f81798"))
