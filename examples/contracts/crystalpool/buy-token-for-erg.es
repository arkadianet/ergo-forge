{	
	def getBuyerPk(box: Box)               = box.R4[Coll[SigmaProp]].getOrElse(Coll[SigmaProp](sigmaProp(false),sigmaProp(false)))(0)
	def getPoolPk(box: Box)                = box.R4[Coll[SigmaProp]].getOrElse(Coll[SigmaProp](sigmaProp(false),sigmaProp(false)))(1)
	def unlockHeight(box: Box)             = box.R5[Int].get
	def getTokenId(box: Box)               = box.R6[Coll[Byte]].getOrElse(Coll[Byte]()) 
	def getBuyRate(box: Box)               = box.R7[Long].get
	def getBuyerMultisigAddress(box: Box)  = box.R8[Coll[Byte]].get

	def tokenId(box: Box) = 
		box.tokens(0)._1
	def tokenAmount(box: Box) = 
		box.tokens(0)._2

	def isSameContract(box: Box) = 
		box.propositionBytes == SELF.propositionBytes

	def isSameTokenId (box: Box) = 
		getTokenId(SELF) == getTokenId(box)

	def includesToken(box: Box) = 
		getTokenId(SELF) == getTokenId(box) &&
		box.tokens.size > 0 &&
		getTokenId(SELF) == tokenId(box)

  	def isGreaterZeroRate(box:Box) =
		getBuyRate(box) > 0

	def isSameBuyer(box: Box)   = 
		getBuyerPk(SELF) == getBuyerPk(box) &&
		getPoolPk(SELF) == getPoolPk(box)

  	def isSameUnlockHeight(box: Box)  = 
		unlockHeight(SELF) == unlockHeight(box)

  	def isSameMultisig(box: Box)    =
		getBuyerMultisigAddress(SELF) == getBuyerMultisigAddress(box)

	def isLegitBuyOrderInput(box: Box) =
		isSameBuyer(box) &&
		isSameUnlockHeight(box) && 
		isSameTokenId(box) &&
		isGreaterZeroRate(box) &&
		isSameMultisig(box) &&
		isSameContract(box)

	val minBuyRate = INPUTS
		.filter(isLegitBuyOrderInput)
		.fold(0L, {(r:Long, box:Box) => {
			if(r < getBuyRate(box)) r else getBuyRate(box)
		}})

  	def isLegitBuyOrderOutput(box: Box) =
		isLegitBuyOrderInput(box)&&
		minBuyRate == getBuyRate(box)

	def isPaymentBox(box:Box) =
		isSameBuyer(box) &&
		isSameUnlockHeight(box) &&
		includesToken(box) &&
		getBuyerMultisigAddress(SELF) == box.propositionBytes

	def sumValuesIn(boxes: Coll[Box]): Long = boxes
			.filter(isLegitBuyOrderInput) 
			.fold(0L, {(a:Long, b: Box) => a + b.value})

	def sumValuesOut(boxes: Coll[Box]): Long = boxes
		.filter(isLegitBuyOrderOutput) 
		.fold(0L, {(a:Long, b: Box) => a + b.value})

	def sumAmountsIn(boxes: Coll[Box]): Long = boxes
		.filter(isLegitBuyOrderInput) 
		.fold(0L, {(a:Long, b: Box) => a + b.value/getBuyRate(b)})

	def sumAmountsOut(boxes: Coll[Box]): Long = boxes
		.filter(isLegitBuyOrderOutput) 
		.fold(0L, {(a:Long, b: Box) => a + b.value/getBuyRate(b)})

  	val valuesIn: Long  = sumValuesIn(INPUTS)
  	val amountsIn: Long = sumAmountsIn(INPUTS)

  	val valuesOut: Long = sumValuesOut(OUTPUTS) 
  	val amountsOut: Long = sumAmountsOut(OUTPUTS) 

  	val deltaAmounts = amountsIn - amountsOut
  	val deltaValues = valuesIn - valuesOut

  	def tokensBought(boxes: Coll[Box]): Long = boxes
		.filter(isPaymentBox) 
		.fold(0L, {(a:Long, b: Box) => a + tokenAmount(b)})

  	val sentToBuyer = tokensBought(OUTPUTS)
  	val isBuyerPaid = deltaAmounts <= sentToBuyer 

	if(HEIGHT > unlockHeight(SELF)){
		getBuyerPk(SELF)
	}else{
		getBuyerPk(SELF) && getPoolPk(SELF) || sigmaProp(isBuyerPaid) && getPoolPk(SELF)
	}
}
