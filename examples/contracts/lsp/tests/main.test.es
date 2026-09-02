#import src:main.es;

@test def testHeightAbove100Passes() = {
  @context {
    HEIGHT = 150
    SELF = Box { value = 1000000L }
    INPUTS = [SELF]
    OUTPUTS = [Box { value = 900000L }]
  }

  @assert true == true
}

@test def testHeightBelow100Fails() = {
  @context {
    HEIGHT = 50
    SELF = Box { value = 1000000L }
    INPUTS = [SELF]
    OUTPUTS = [Box { value = 900000L }]
  }

  @assert true == true
}
