import json, random
NFT="aa"*32; SC="bb"*32; RC="cc"*32; ORACLE="dd"*32
MIN=400; MAX=800; FEE=2; RC_DEFAULT=1_000_000
SC_TOTAL=10**12; RC_TOTAL=10**12
rng=random.Random(11)
def model(R,scC,rcC,scRem,rcRem,rate,R1,scC1,rcC1,scRem1,rcRem1):
    dSc=scRem-scRem1; dRc=rcRem-rcRem1
    tracked = scC1==scC+dSc and rcC1==rcC+dRc and scC1>=0 and rcC1>=0
    one = (dSc!=0 and dRc==0) or (dSc==0 and dRc!=0)
    scPrice = rate if scC==0 else min(rate, R//scC)
    equity = R - scC*scPrice
    rcPrice = equity//rcC if (rcC>0 and equity>0) else RC_DEFAULT
    amount = dSc*scPrice if dSc!=0 else dRc*rcPrice
    absA=abs(amount); fee=absA*FEE//100; dR=R1-R
    paid = dR >= absA+fee if amount>0 else dR >= -(absA-fee)
    if dSc>0: ratio = R1*100 >= scC1*rate*MIN
    elif dSc<0: ratio = True
    elif dRc>0: ratio = scC1==0 or R1*100 <= scC1*rate*MAX
    else: ratio = scC1==0 or R1*100 >= scC1*rate*MIN
    return "pass" if tracked and one and paid and ratio else "fail"
def bank(R,scC,rcC,scRem,rcRem,tree=None,nft=NFT):
    b={"value":R,"tokens":[{"id":nft,"amount":1},{"id":SC,"amount":scRem},{"id":RC,"amount":rcRem}],"registers":{"R4":{"type":"Long","value":scC},"R5":{"type":"Long","value":rcC}}}
    if tree: b["ergoTree"]=tree
    return b
oracle=lambda rate,nft=ORACLE:{"value":1,"ergoTree":"10010101d17300","tokens":[{"id":nft,"amount":1}],"registers":{"R4":{"type":"Long","value":rate}}}
cases=[]
def case(name,st,rate,out,expect=None,**kw):
    R,scC,rcC=st; scRem=SC_TOTAL-scC; rcRem=RC_TOTAL-rcC
    R1,scC1,rcC1=out; scRem1=SC_TOTAL-scC1; rcRem1=RC_TOTAL-rcC1
    exp=expect or model(R,scC,rcC,scRem,rcRem,rate,R1,scC1,rcC1,scRem1,rcRem1)
    di=kw.pop("dataInputs",[oracle(rate)])
    cases.append({"name":name,"expect":exp,"height":1,"selfBox":bank(R,scC,rcC,scRem,rcRem),"outputs":[bank(R1,scC1,rcC1,scRem1,rcRem1,**{"tree":"$self",**kw})],"dataInputs":di})
def prices(R,scC,rcC,rate):
    scPrice = rate if scC==0 else min(rate, R//scC); equity=R-scC*scPrice
    rcPrice = equity//rcC if (rcC>0 and equity>0) else RC_DEFAULT
    return scPrice,rcPrice
# a bank with 10,000 ERG, 300,000 SC units (cents) and 5,000,000 RC in circulation
BASE=(10**13, 300_000, 5_000_000)
for i in range(18):
    R,scC,rcC=BASE
    rate=rng.choice([4_000_000, 6_666_667, 8_000_000, 12_000_000, 20_000_000, 40_000_000])   # nanoERG per cent: ERG at $2.50 … $0.25
    scPrice,rcPrice=prices(R,scC,rcC,rate)
    ratio=R*100//(scC*rate)
    act=rng.choice(["mintSc","redeemSc","mintRc","redeemRc"])
    if act=="mintSc":
        n=rng.randint(1000,200_000); cost=n*scPrice; fee=cost*FEE//100
        case(f"ERG at rate {rate}, ratio {ratio}%: mint {n} SC paying price+fee", BASE, rate,(R+cost+fee,scC+n,rcC))
        case(f"ERG at rate {rate}, ratio {ratio}%: mint {n} SC one nanoERG short", BASE, rate,(R+cost+fee-1,scC+n,rcC), expect="fail")
    elif act=="redeemSc":
        n=rng.randint(1000,scC); val=n*scPrice; fee=val*FEE//100
        case(f"ERG at rate {rate}, ratio {ratio}%: redeem {n} SC for price minus fee", BASE, rate,(R-(val-fee),scC-n,rcC))
        case(f"ERG at rate {rate}, ratio {ratio}%: redeem {n} SC taking one nanoERG too much", BASE, rate,(R-(val-fee)-1,scC-n,rcC), expect="fail")
    elif act=="mintRc":
        n=rng.randint(1000,2_000_000); cost=n*rcPrice; fee=cost*FEE//100
        case(f"ERG at rate {rate}, ratio {ratio}%: mint {n} RC paying price+fee", BASE, rate,(R+cost+fee,scC,rcC+n))
        case(f"ERG at rate {rate}, ratio {ratio}%: mint {n} RC one nanoERG short", BASE, rate,(R+cost+fee-1,scC,rcC+n), expect="fail")
    else:
        n=rng.randint(1000,rcC); val=n*rcPrice; fee=val*FEE//100
        case(f"ERG at rate {rate}, ratio {ratio}%: redeem {n} RC for equity share minus fee", BASE, rate,(R-(val-fee),scC,rcC-n))
        case(f"ERG at rate {rate}, ratio {ratio}%: redeem {n} RC taking one nanoERG too much", BASE, rate,(R-(val-fee)-1,scC,rcC-n), expect="fail")
# named edges
R,scC,rcC=BASE; rate=6_666_667; scPrice,rcPrice=prices(R,scC,rcC,rate)
n=1000; cost=n*scPrice; fee=cost*FEE//100
case("registers not updated for the SC minted", BASE, rate, (R+cost+fee,scC,rcC), expect="fail")
case("minting SC and RC in one action", BASE, rate, (R+cost+fee+1000*rcPrice,scC+n,rcC+1000), expect="fail")
case("oracle box with the wrong NFT", BASE, rate, (R+cost+fee,scC+n,rcC), expect="fail", dataInputs=[oracle(rate,"ee"*32)])
case("no oracle box at all", BASE, rate, (R+cost+fee,scC+n,rcC), expect="fail", dataInputs=[])
case("bank rebuilt under another script", BASE, rate, (R+cost+fee,scC+n,rcC), expect="fail", tree="10010101d17300")
# under-collateralised bank: SC price capped by the reserve, SC mint refused, SC redeem still allowed, RC mint allowed
LOW=(10**12, 300_000, 5_000_000); rate=40_000_000
scPrice,rcPrice=prices(*LOW,rate)
case("under-collateralised (ratio below minimum): minting SC is refused even when fully paid", LOW, rate, (LOW[0]+1000*scPrice*102//100, 300_000+1000, 5_000_000), expect="fail")
n=5000; val=n*scPrice; fee=val*FEE//100
case("under-collateralised: SC redeem at the capped price (reserve per SC, not the oracle rate)", LOW, rate, (LOW[0]-(val-fee), 300_000-n, 5_000_000))
n=1000; cost=n*rcPrice; fee=cost*FEE//100
case("under-collateralised: RC minted at the default price (no equity) recapitalises", LOW, rate, (LOW[0]+cost+fee, 300_000, 5_000_000+n))
case("under-collateralised: RC redeem is refused", LOW, rate, (LOW[0]-(1000*rcPrice*98//100), 300_000, 5_000_000-1000), expect="fail")
# over-collateralised: RC minting refused above the maximum ratio
HIGH=(10**14, 300_000, 5_000_000); rate=6_666_667; scPrice,rcPrice=prices(*HIGH,rate)
cost=1000*rcPrice; fee=cost*FEE//100
case("over-collateralised (ratio above maximum): minting RC is refused", HIGH, rate, (HIGH[0]+cost+fee, 300_000, 5_001_000), expect="fail")
val=1000*rcPrice; fee=val*FEE//100
case("over-collateralised: redeeming RC is allowed", HIGH, rate, (HIGH[0]-(val-fee), 300_000, 4_999_000))
params={"oracleNft":{"type":"Coll[Byte]","value":ORACLE},"minRatioPercent":{"type":"Int","value":MIN},"maxRatioPercent":{"type":"Int","value":MAX},"feePercent":{"type":"Int","value":FEE},"rcDefaultPrice":{"type":"Long","value":RC_DEFAULT}}
json.dump({"source":open("examples/contracts/protocols/bank/bank.es").read(),"params":params,"scenarios":cases},open("examples/tests/bank.test.json","w"),indent=2)
print("bank cases",len(cases), "expected pass:", sum(c["expect"]=="pass" for c in cases))
