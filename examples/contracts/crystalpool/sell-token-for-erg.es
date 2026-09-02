{	
	def getSellerPk(box: Box)              = box.R4[Coll[SigmaProp]].getOrElse(Coll[SigmaProp](sigmaProp(false),sigmaProp(false)))(0)
	def getPoolPk(box: Box)                = box.R4[Coll[SigmaProp]].getOrElse(Coll[SigmaProp](sigmaProp(false),sigmaProp(false)))(1)
	def unlockHeight(box: Box)             = box.R5[Int].get
	def getTokenId(box: Box)               = box.R6[Coll[Byte]].getOrElse(Coll[Byte]()) 
	def getSellRate(box: Box)              = box.R7[Long].get
	def getSellerMultisigAddress(box: Box) = box.R8[Coll[Byte]].get

 	def tokenId(box: Box) = box.tokens(0)._1
	def tokenAmount(box: Box) = box.tokens(0)._2
  
	def isSameContract(box: Box) = 
		box.propositionBytes == SELF.propositionBytes
  
	def isSameToken(box: Box)    = 
	  	getTokenId(SELF) == getTokenId(box) &&
	  	box.tokens.size > 0 &&
		getTokenId(SELF) == tokenId(box)

  	def isGreaterZeroRate(box:Box) =
    	getSellRate(box) > 0
  
	def isSameSeller(box: Box)   = 
    	getSellerPk(SELF) == getSellerPk(box) &&
    	getPoolPk(SELF) == getPoolPk(box)

  	def isSameUnlockHeight(box: Box)  = 
    	unlockHeight(SELF) == unlockHeight(box)

  	def isSameMultisig(box: Box)    =
    	getSellerMultisigAddress(SELF) == getSellerMultisigAddress(box)

	def isLegitInputBox(b: Box) = {
	    isSameContract(b) && 
    	isSameToken(b) && 
    	isSameMultisig(b) && 
    	isSameSeller(b) && 
    	isGreaterZeroRate(b)
	}
  
	def isPaymentBox(box:Box) = {
		isSameSeller(box) &&
    	isSameUnlockHeight(box) &&
		getTokenId(SELF) == getTokenId(box) &&
		getSellerMultisigAddress(SELF) == box.propositionBytes
	}
  
	def sumTokensIn(boxes: Coll[Box]): Long = boxes
		.filter(isLegitInputBox) 
		.fold(0L, {(a:Long, b: Box) => a + b.tokens(0)._2})
  
	val tokensIn: Long = sumTokensIn(INPUTS)
  
	val avgRateInputs: Long = INPUTS
    	.filter(isLegitInputBox)
    	.fold(0L, {(a:Long, b: Box) => {
    	  a + getSellRate(b)*tokenAmount(b)
    	}}) / tokensIn 
	
	val maxSellRate = INPUTS
    	.filter(isLegitInputBox)
    	.fold(0L, {(r:Long, box:Box) => {
		    if(r > getSellRate(box)) r else getSellRate(box)
		}})
  
	def sumTokensInAtMaxRate(boxes: Coll[Box]): Long = boxes
		.filter(isLegitInputBox)
		.filter({(b: Box)=> getSellRate(b) == maxSellRate})
		.fold(0L, {(a:Long, b: Box) => a + tokenAmount(b)})
  
	def isMaxRateChangeBox(box: Box) = {
		isSameSeller(box) &&
		isSameUnlockHeight(box) &&
		isSameToken(box) &&
		maxSellRate == getSellRate(box) &&
		isSameMultisig(box) &&
		isSameContract(box)
	}
  
	def tokensRemaining(boxes: Coll[Box]): Long = boxes
		.filter(isMaxRateChangeBox)
		.fold(0L, {(a:Long, b: Box) => a + tokenAmount(b)}) 
	
	val tokensBack: Long = tokensRemaining(OUTPUTS)
	val tokensSold: Long = tokensIn - tokensBack
  
	val nanoErgsPaid: Long = OUTPUTS
		.filter(isPaymentBox)
		.fold(0L, {(a:Long, b: Box) => a + b.value})
  
  	val valueOfSoldTokens: Long  = tokensIn * avgRateInputs - tokensBack * maxSellRate
  	val amountOfSoldTokens: Long = tokensIn - tokensBack
	val avgTokenPrice: Long =  valueOfSoldTokens / amountOfSoldTokens

	val tokensInputAtMaxRate = sumTokensInAtMaxRate(INPUTS) 
	val sellOrderChangeBoxIsFine = tokensInputAtMaxRate > tokensBack 
	val sellerPaid: Boolen = tokensSold * avgTokenPrice <= nanoErgsPaid
  
	val orderFilled = sellerPaid && sellOrderChangeBoxIsFine
  
	if(HEIGHT > unlockHeight(SELF)){
		getSellerPk(SELF)
	}else{
		getSellerPk(SELF) && getPoolPk(SELF) || sigmaProp(orderFilled) && getPoolPk(SELF)
	}
}