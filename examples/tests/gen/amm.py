import json, random
A="028333f9f7454f8d5ff73dbac9833767ed6fc3a86cf0a73df946b32ea9927d9197"
NFT="aa"*32; LP="cc"*32; Y="bb"*32
LP_SUPPLY=10**12; FEE_NUM=997; FEE_DENOM=1000
X0=10**12; Y0=5*10**9; LPREM0=LP_SUPPLY-10**9   # 1000 ERG, 5e9 units of Y, 1e9 LP circulating
rng=random.Random(7)
def model(X,Y,lprem,X1,Y1,lprem1,preserved=True):
    supply=LP_SUPPLY-lprem; dS=lprem-lprem1; dX=X1-X; dY=Y1-Y
    if dX>0: swap = dS==0 and Y*dX*FEE_NUM >= -dY*(X*FEE_DENOM+dX*FEE_NUM)
    else:    swap = dS==0 and X*dY*FEE_NUM >= -dX*(Y*FEE_DENOM+dY*FEE_NUM)
    dep = dS>0 and dX>0 and dY>0 and dS*X<=dX*supply and dS*Y<=dY*supply
    red = dS<0 and dX<0 and dY<0 and (-dX)*supply<=(-dS)*X and (-dY)*supply<=(-dS)*Y
    return "pass" if preserved and (swap or dep or red) else "fail"
def box(value,lprem,yamt,tree="$self",nft=NFT,lpid=LP,yid=Y,ntokens=3):
    toks=[{"id":nft,"amount":1},{"id":lpid,"amount":lprem},{"id":yid,"amount":yamt}][:ntokens]
    b={"value":value,"tokens":toks}
    if tree: b["ergoTree"]=tree
    return b
cases=[]
def case(name,X1,Y1,lprem1,expect=None,**kw):
    exp = expect or model(X0,Y0,LPREM0,X1,Y1,lprem1)
    cases.append({"name":name,"expect":exp,"height":1,"selfBox":box(X0,LPREM0,Y0,tree=None),"outputs":[box(X1,lprem1,Y1,**kw)]})
# swaps: sell X for Y
for i in range(8):
    dX=rng.randint(10**8, X0//3)
    out=(Y0*dX*FEE_NUM)//(X0*FEE_DENOM+dX*FEE_NUM)
    case(f"swap {dX/1e9:.3f} ERG in, the most Y the curve allows out ({out})", X0+dX, Y0-out, LPREM0)
    case(f"swap {dX/1e9:.3f} ERG in, one unit of Y too many out", X0+dX, Y0-out-1, LPREM0)
# swaps: sell Y for X
for i in range(8):
    dY=rng.randint(10**6, Y0//3)
    out=(X0*dY*FEE_NUM)//(Y0*FEE_DENOM+dY*FEE_NUM)
    case(f"swap {dY} Y in, the most ERG the curve allows out ({out/1e9:.6f})", X0-out, Y0+dY, LPREM0)
    case(f"swap {dY} Y in, one nanoERG too many out", X0-out-1, Y0+dY, LPREM0)
# deposits
supply=LP_SUPPLY-LPREM0
for i in range(6):
    dX=rng.randint(10**9, X0//2); dY=rng.randint(10**6, Y0//2)
    mx=min(dX*supply//X0, dY*supply//Y0)
    case(f"deposit {dX/1e9:.2f} ERG + {dY} Y, the proportional LP minted ({mx})", X0+dX, Y0+dY, LPREM0-mx)
    case(f"deposit {dX/1e9:.2f} ERG + {dY} Y, one LP too many minted", X0+dX, Y0+dY, LPREM0-mx-1)
# redeems
for i in range(6):
    dS=rng.randint(10**6, supply//2)
    ox=dS*X0//supply; oy=dS*Y0//supply
    case(f"redeem {dS} LP for the proportional reserves ({ox/1e9:.4f} ERG, {oy} Y)", X0-ox, Y0-oy, LPREM0+dS)
    case(f"redeem {dS} LP taking one nanoERG too many", X0-ox-1, Y0-oy, LPREM0+dS)
    case(f"redeem {dS} LP taking one Y too many", X0-ox, Y0-oy-1, LPREM0+dS)
# structure
dX=10**10; out=(Y0*dX*FEE_NUM)//(X0*FEE_DENOM+dX*FEE_NUM)
case("a fair swap that also mints LP for free", X0+dX, Y0-out, LPREM0-1, expect="fail")
case("a fair swap into a box under another script", X0+dX, Y0-out, LPREM0, expect="fail", tree="10010101d17300")
case("a fair swap that swaps the NFT for another token", X0+dX, Y0-out, LPREM0, expect="fail", nft="dd"*32)
case("a fair swap that changes the LP token id", X0+dX, Y0-out, LPREM0, expect="fail", lpid="dd"*32)
case("a successor with only two tokens", X0+dX, Y0-out, LPREM0, expect="fail", ntokens=2)
case("a deposit that takes ERG out", X0-1, Y0+10**6, LPREM0-1, expect="fail")
cases.append({"name":"no outputs at all","expect":"fail","height":1,"selfBox":box(X0,LPREM0,Y0,tree=None)})
params={"lpSupply":{"type":"Long","value":LP_SUPPLY},"feeNum":{"type":"Int","value":FEE_NUM},"feeDenom":{"type":"Int","value":FEE_DENOM}}
json.dump({"source":open("examples/contracts/protocols/amm/pool.es").read(),"params":params,"scenarios":cases},open("examples/tests/amm-pool.test.json","w"),indent=2)
print("pool cases",len(cases))
# ── swap order
p2pk=lambda k:"0008cd"+k
poolbox=lambda nft=NFT:{"value":X0,"ergoTree":"10010101d17300","tokens":[{"id":nft,"amount":1},{"id":LP,"amount":LPREM0},{"id":Y,"amount":Y0}]}
def order(name,expect,outputs,pool=poolbox(),residual=None):
    c={"name":name,"expect":expect,"height":1,"selfIndex":1,"inputs":[pool,{"value":10**9}],"outputs":outputs}
    if residual: c["expectResidual"]=residual
    return c
paid=lambda n,tok=Y:{"value":10**6,"ergoTree":p2pk(A),"tokens":[{"id":tok,"amount":n}]}
succ=poolbox()
ocases=[
 order("filled at the minimum: no key needed",'pass',[succ,paid(100)]),
 order("filled above the minimum",'pass',[succ,paid(150)]),
 order("one unit short: only the trader could sign that",'needsProof',[succ,paid(99)],residual=A[:8]),
 order("paid in the wrong token",'needsProof',[succ,paid(100,"dd"*32)],residual=A[:8]),
 order("against a box that is not the pool (no NFT)",'needsProof',[succ,paid(100)],pool=poolbox("ee"*32),residual=A[:8]),
 order("the trader cancels",'needsProof',[],residual=A[:8]),
]
oparams={"trader":{"type":"SigmaProp","value":A},"poolNft":{"type":"Coll[Byte]","value":NFT},"tokenY":{"type":"Coll[Byte]","value":Y},"minOutput":{"type":"Long","value":100}}
json.dump({"source":open("examples/contracts/protocols/amm/swap-order.es").read(),"params":oparams,"scenarios":ocases},open("examples/tests/amm-swap-order.test.json","w"),indent=2)
print("order cases",len(ocases))
