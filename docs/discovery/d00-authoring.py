import json,pathlib,copy
root=pathlib.Path('ergo-sandbox/tests/fixtures/properties')
assert not root.joinpath('manifest.json').exists(), 'Accepted inventory is immutable; use a new version'
sources={'reserve':'sigmaProp(OUTPUTS(0).value >= 1000000L)','issuance':'sigmaProp(OUTPUTS.size >= 1)','continuation':'sigmaProp(SELF.value >= 1000000L)','bounded_response':'sigmaProp(OUTPUTS(0).R4[Long].isDefined)','funding':'sigmaProp(true)'}
def box(kind,value=10000000,reg=0,tokens=None): return {'sourceId':kind,'value':value,'registers':[{'type':'Long','value':reg}], 'tokens':tokens or []}
def role(collection,position=None,script=None,cardinality='exactly-one'):
    return dict(collection=collection,selector={'position':position} if position is not None else {'sourceId':script},cardinality=cardinality)
def read(role,field,unit):return {'op':'read','role':role,'field':field,'unit':unit}
def lit(n,unit):return {'op':'literal','value':str(n),'unit':unit}
def op(name,*args):return {'op':name,'args':list(args)}
cases=[]; answers=[]
def add(id,family,inputs,steps,roles,assertion,operands,verdict,explanation,guard=True,scope='transition',**kw):
    prop={'draftVocabulary':'author-property-design:v1','id':id,'revision':1,'scope':scope,'roles':roles,'guard':{'op':'boolean','value':guard},'assertion':assertion,'authorizationPremises':['No signature identity is inferred. These synthetic keyless contracts deliberately do not enforce the entire author assertion.'],'sourceReferences':[family],**kw}
    c={'id':id,'family':family,'inputs':inputs,'steps':steps,'property':prop,'rootPremises':'Explicit hypothetical external UTXOs; no historical availability or validated initialization. Each step has a separately supplied context and state.','publicationStatus':'public-synthetic-authored','exposure':'Author supplied all witnesses and answers before node/extraction inspection; regression only, no discovery eligibility.'}
    cases.append(c); answers.append({'id':id,'family':family,'expectedReferenceAcceptance':['accepted']*len(steps),'expectedBindings':copy.deepcopy(roles),'expectedGuard':guard,'operands':operands,'expectedDisposition':verdict,'explanation':explanation,'defectAttribution':'unadjudicated author assertion; a refutation alone does not establish a contract defect','reviewStatus':'independent-human-review-missing'})
for family in ['reserve','issuance','continuation','bounded_response']:
 for j in range(4):
    id=family+('_violation_' if j<2 else '_control_')+str(j%2+1)
    verdict='violated' if j<2 else 'holds-on-execution'
    if family=='reserve':
      after,liab=[(10000000,7000000),(9000000,5000000),(12000000,7000000),(10000000,5000000)][j]
      roles={'before':role('inputs',0),'after':role('outputs',0)}
      assertion=op('ge',op('sub',read('after','value','nanoERG'),read('before','value','nanoERG')),op('sub',read('after','R4:Long','nanoERG'),read('before','R4:Long','nanoERG')))
      add(id,family,[box(family,reg=5000000),box('funding')],[[box(family,after,liab),box('funding',20000000-after)]],roles,assertion,{'reserveBefore':10000000,'reserveAfter':after,'liabilityBefore':5000000,'liabilityAfter':liab,'left':after-10000000,'right':liab-5000000,'units':'nanoERG; R4 declares fixed-scale liabilities at 1 nanoERG per unit'},verdict,'Compare reserve increase to fixed-scale liability increase; explicit external funding and change are excluded from the reserve role. The contract enforces only a minimum output value.')
    elif family=='issuance':
      amount,quota=[(11,10),(21,20),(10,10),(19,20)][j]
      roles={'mint':role('outputs',0)}
      assertion=op('le',read('mint','token:first-input-id','token-units'),lit(quota,'token-units'))
      add(id,family,[box(family)],[[box(family,tokens=[{'id':'first-input-id','amount':amount}])]],roles,assertion,{'inputSupply':0,'outputSupply':amount,'delta':amount,'quota':quota,'authority':'No required authority input for this quota declaration; node issuance uses first spending input ID.'},verdict,'This is accepted over-issuance versus an author quota, not a claim about human consent or unauthorized minting. Zero previous supply is explicit in the transaction input inventory.')
    elif family=='continuation':
      n=[0,2,1,1][j]; value=12000000 if j==3 else 10000000
      outs=([box('funding',value)] if n==0 else [box(family,value//n,7)]*n)
      roles={'state':role('inputs',0),'successors':role('outputs',script=family,cardinality='all-matches')}
      assertion=op('and',op('eq',{'op':'count','role':'successors','unit':'count'},lit(1,'count')),{'op':'all-equal','role':'successors','field':'R4:Long','value':'7','unit':'state-tag'})
      # all-equal shorthand is expanded by D01 authorship; avoid undefined collection operator: count plus exactly-one read.
      roles['successor']=role('outputs',script=family)
      assertion=op('and',op('eq',{'op':'count','role':'successors','unit':'count'},lit(1,'count')),op('eq',read('state','R4:Long','state-tag'),lit(7,'state-tag')))
      # Cardinality failure alone is false; no exactly-one binding needed to score it.
      del roles['successor']
      add(id,family,[box(family,value,7)],[outs],roles,assertion,{'inputStateTag':7,'successorCount':n,'requiredCount':1,'successorScripts':[family]*n,'successorRegisterTags':[7]*n,'left':n,'right':1},verdict,'Spending the explicit state input requires exactly one output with the same exact script; the state input has R4=7. The permissive contract checks only input value. No NFT uniqueness is inferred.')
    else:
      goals=[[1,1,1],[1,1,1],[1,0,0],[1,1,0]][j]
      roles={'state':role('inputs',0),'successor':role('outputs',0)}
      add(id,family,[box(family,10000000+j*1000000,1)],[[box(family,10000000+j*1000000,g)] for g in goals],roles,op('eq',read('successor','R4:Long','request-state'),lit(0,'request-state')),{'triggerIndex':0,'triggerPending':1,'K':2,'goalAtSuccessors': [g==0 for g in goals[1:]],'R4ByOutput':goals,'firstGoalIndex': next((i for i,g in enumerate(goals) if i>0 and g==0),None)},verdict,'At the pending trigger, promise service within two subsequent accepted actions. R4 zero means serviced. All three steps must later pass D02 linked checking; individual D00 acceptance is insufficient.',scope='bounded-response',trigger=op('eq',read('successor','R4:Long','request-state'),lit(1,'request-state')),horizon=2,schedule={'height':[100,101,102],'action':['request','tick','tick'],'restriction':'One supplied transaction per hypothetical block; no fairness or eventual-service premise.'})
base=copy.deepcopy(cases[3]); baseanswer=copy.deepcopy(answers[3])
for name in ['missing_role','ambiguous_role','wrong_register_type','arithmetic_overflow','false_guard','short_response_horizon','unbounded_eventuality','unsupported_dynamic_expression']:
    c=copy.deepcopy(cases[12] if name in ['short_response_horizon','unbounded_eventuality'] else base); a=copy.deepcopy(answers[12] if name in ['short_response_horizon','unbounded_eventuality'] else baseanswer)
    c['id']=name;c['family']='boundary';c['property']['id']=name;a['id']=name;a['family']='boundary';a['expectedDisposition']='unresolved'
    if name=='missing_role': c['property']['roles']['after']=role('outputs',9); a['operands']={'afterMatches':[]}; a['explanation']='Required exactly-one output position 9 is absent; never default its value to zero.'
    elif name=='ambiguous_role':
      c['steps'][0][1]['sourceId']='reserve';c['property']['roles']['after']=role('outputs',script='reserve');a['operands']={'afterMatches':[0,1]};a['explanation']='Both outputs have the exact reserve script; exactly-one is unresolved, never select the first.'
    elif name=='wrong_register_type': c['steps'][0][0]['registers']=[{'type':'Boolean','value':True}];a['operands']={'required':'Long','actual':'Boolean','field':'outputs[0].R4'};a['explanation']='R4 exists with Boolean type; a Long read is unresolved.'
    elif name=='arithmetic_overflow': c['property']['assertion']=op('ge',op('mul-literal',read('after','value','nanoERG'),lit(2**127-1,'scalar')),lit(0,'nanoERG'));a['operands']={'leftInput':'10000000','multiplier':str(2**127-1),'mathematicalProduct':str(10000000*(2**127-1)),'ceiling':str(2**127-1)};a['explanation']='Multiplication exceeds signed 128-bit result ceiling; do not wrap or report false.'
    elif name=='false_guard': c['property']['guard']={'op':'boolean','value':False};a['expectedGuard']=False;a['expectedDisposition']='not-applicable';a['operands']={'guard':False,'assertionNotEvaluated':True};a['explanation']='Inactive guard is not a nonviolating control and gives no applicable coverage.'
    elif name=='short_response_horizon': c['steps']=c['steps'][:2];c['property']['schedule']['height']=[100,101];c['property']['schedule']['action']=['request','tick'];a['operands']={'K':2,'subsequentActions':1,'goalObserved':False};a['explanation']='Only one of two required successor actions exists; incomplete horizon cannot refute response.'
    elif name=='unbounded_eventuality': c['property']['scope']='eventual';c['property']['horizon']=None;a['expectedDisposition']='unsupported-property';a['operands']={'horizon':None};a['explanation']='An unbounded eventual-service promise requires semantics outside finite v1, not timeout evidence.'
    else:c['property']['assertion']={'op':'execute-extension','input':0,'variable':0};a['expectedDisposition']='unsupported-property';a['operands']={'operator':'execute-extension'};a['explanation']='Dynamic expression execution is outside the finite property vocabulary, regardless of node acceptance.'
    a['expectedBindings']=copy.deepcopy(c['property']['roles']);a['expectedReferenceAcceptance']=['accepted']*len(c['steps']);cases.append(c);answers.append(a)
root.joinpath('authored-inputs.json').write_text(json.dumps({'version':'property-authoring:v1','sources':sources,'cases':cases},indent=2)+'\n')
root.joinpath('expected.json').write_text(json.dumps({'version':'property-expected:v1','authorship':'Authored by Codex separately from any property evaluator, before node or extraction output inspection; not independent human adjudication.','cases':answers},indent=2)+'\n')
root.joinpath('transfer-registration.json').write_text(json.dumps({'version':'property-transfer:v1','plannedCases':2,'denominator':None,'cases':[],'nominationCeilingPersonHours':4,'operatorExposure':['Existing local P05 sale and USE fixture inventories inspected. Sale is already an extraction reference; no pair of independently grounded non-extraction guarantees and violating/control executions was supplied.','All synthetic cases and their answers are author-exposed; none qualifies as transfer or discovery.'],'reviewer':None,'semanticMeasurement':'incomplete-independent-review-missing','utilityPass':False,'reason':'No defensible two-case transfer pair available under current local evidence and publication scope. Do not relabel synthetic or extraction cases to fill it.','checkpointConsequence':'Cancel capabilities 2 and 3 for this arc at the checkpoint; capability 1 regression tooling may remain if correctness gates pass.','authorHoursPerCase':None,'reviewerHoursPerCase':None,'effortAccounting':'No independent reviewer effort supplied; no invented per-case timing.'},indent=2)+'\n')
