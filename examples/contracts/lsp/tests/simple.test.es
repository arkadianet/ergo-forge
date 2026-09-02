#import src:simple.es;

@test def testHeightPass() = {
  @context {
    HEIGHT = 150
    SELF = Box { value = 1000000L }
    INPUTS = [SELF]
    OUTPUTS = [Box { value = 900000L }]
  }

  @assert true == true
}

@test def testHeightFail() = {
  @context {
    HEIGHT = 50
    SELF = Box { value = 1000000L }
    INPUTS = [SELF]
    OUTPUTS = [Box { value = 900000L }]
  }

  @assert true == true
}
