{
	def getSellerPk(box: Box)              = box.R4[Coll[SigmaProp]].get(0)
	def getPoolPk(box: Box)                = box.R4[Coll[SigmaProp]].get(1)
	def unlockHeight(box: Box)             = box.R5[Int].get
	
	if(HEIGHT > unlockHeight(SELF)){
		getSellerPk(SELF)
	}else{
		getSellerPk(SELF) // && getPoolPk(SELF)
	}
}