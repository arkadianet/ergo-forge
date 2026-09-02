#import src:template_test.es;

@test def testTemplateCompiles() = {
  @context {
    HEIGHT = 150
    SELF = Box { value = 1000000L }
    INPUTS = [SELF]
    OUTPUTS = [Box { value = 900000L }]
  }

  @assert true == true
}
