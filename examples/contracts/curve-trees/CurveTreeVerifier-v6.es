{
    // =========================================================================
    // Curve Trees on Ergo — v6: The Final Mainnet-Ready Accumulator
    // =========================================================================
    // 
    // Key fixes over v5:
    //   1. 16-byte chunking (b2U helper) bypasses the two's complement trap
    //      where byteArrayToBigInt(32 bytes with high bit) gives negative BigInt,
    //      and .toUnsignedMod(p2) produces (X - 2^256) mod p2 ≠ X (since p2 ≠ 2^256)
    //   2. All fold accumulators and comparisons use strictly UnsignedBigInt
    //
    // Register layout:
    //   R4: Coll[Byte] — tree root x-coordinate (32 bytes)
    //   R5: GroupElement — G1 for E1 (secp256k1)
    //   R6: GroupElement — G2 for E1 (secp256k1)
    //   R7: Coll[Byte] — E2 generators: g1x|g1y|g2x|g2y (128 bytes)
    //   R8: Coll[Byte] — G12 precomputed: g12x|g12y (64 bytes)
    //
    // Context extensions:
    //   Var(0): Coll[Byte] — leaf scalar (32 bytes)
    //   Var(1): Coll[Byte] — sibling scalars (96 bytes = 3 × 32)
    //   Var(2): Coll[Byte] — path directions (3 bytes)
    //   Var(3): Coll[Byte] — Straus-Shamir bits (255 bytes)
    //   Var(4): Coll[Byte] — Z⁻¹ for prover-assisted inversion (32 bytes)
    // =========================================================================
    
    val rootExpected = SELF.R4[Coll[Byte]].get
    val G1_E1 = SELF.R5[GroupElement].get
    val G2_E1 = SELF.R6[GroupElement].get
    
    val e2g = SELF.R7[Coll[Byte]].get    // 128 bytes: g1x|g1y|g2x|g2y
    val g12b = SELF.R8[Coll[Byte]].get   // 64 bytes: g12x|g12y

    val leafBytes = getVar[Coll[Byte]](0).get
    val sibBytes  = getVar[Coll[Byte]](1).get
    val dirs      = getVar[Coll[Byte]](2).get
    val bits_E2   = getVar[Coll[Byte]](3).get
    val zInvBytes = getVar[Coll[Byte]](4).get

    val p2 = unsignedBigInt("115792089237316195423570985008687907852837564279074904382605163141518161494337")
    val shift128 = unsignedBigInt("340282366920938463463374607431768211456")
    val zeroByte = Coll(0.toByte)

    // SAFELY convert 32 bytes to UnsignedBigInt by splitting into 16-byte positive halves.
    // Bypasses the 33-byte crash limit AND the negative BigInt MSB trap!
    def b2U(b: Coll[Byte]): UnsignedBigInt = {
        val hi = byteArrayToBigInt(zeroByte ++ b.slice(0, 16)).toUnsignedMod(p2)
        val lo = byteArrayToBigInt(zeroByte ++ b.slice(16, 32)).toUnsignedMod(p2)
        hi.multiplyMod(shift128, p2).plusMod(lo, p2)
    }

    val g1x = b2U(e2g.slice(0, 32))
    val g1y = b2U(e2g.slice(32, 64))
    val g2x = b2U(e2g.slice(64, 96))
    val g2y = b2U(e2g.slice(96, 128))
    val g12x = b2U(g12b.slice(0, 32))
    val g12y = b2U(g12b.slice(32, 64))

    // LEVEL 0 → 1: Native secp256k1 (E1)
    val l0_leaf = b2U(leafBytes)
    val l0_sib  = b2U(sibBytes.slice(0, 32))
    val s1_L0 = if (dirs(0) == 1.toByte) l0_leaf else l0_sib
    val s2_L0 = if (dirs(0) == 1.toByte) l0_sib  else l0_leaf
    val p_L0 = G1_E1.expUnsigned(s1_L0).multiply(G2_E1.expUnsigned(s2_L0))
    val out_L0_bytes = p_L0.getEncoded.slice(1, 33)

    // LEVEL 1 → 2: Scalars for E2 fold
    val l1_in  = b2U(out_L0_bytes)
    val l1_sib = b2U(sibBytes.slice(32, 64))
    val s1_L1 = if (dirs(1) == 1.toByte) l1_in else l1_sib
    val s2_L1 = if (dirs(1) == 1.toByte) l1_sib else l1_in

    val uZero = unsignedBigInt("0")
    val uOne  = unsignedBigInt("1")
    val uTwo  = unsignedBigInt("2")
    val uThree= unsignedBigInt("3")
    val uFour = unsignedBigInt("4")
    val uEight= unsignedBigInt("8")

    // Bit reconstruction
    val reconstructed = bits_E2.fold((uZero, uZero), { (acc: (UnsignedBigInt, UnsignedBigInt), b: Byte) =>
        val r1 = acc._1.multiplyMod(uTwo, p2)
        val r2 = acc._2.multiplyMod(uTwo, p2)
        val b1 = if (b == 2.toByte || b == 3.toByte) uOne else uZero
        val b2 = if (b == 1.toByte || b == 3.toByte) uOne else uZero
        (r1.plusMod(b1, p2), r2.plusMod(b2, p2))
    })
    val bitsValid = (reconstructed._1 == s1_L1) && (reconstructed._2 == s2_L1)

    // Initialization from first bit
    val firstB = bits_E2(0)
    val startX = if (firstB == 3.toByte) g12x else if (firstB == 2.toByte) g1x else g2x
    val startY = if (firstB == 3.toByte) g12y else if (firstB == 2.toByte) g1y else g2y

    // Projective Straus-Shamir fold. Point = ((X, Y), Z)
    val sum_L1 = bits_E2.slice(1, bits_E2.size).fold(((startX, startY), uOne), { (acc: ((UnsignedBigInt, UnsignedBigInt), UnsignedBigInt), b: Byte) =>
        val aX = acc._1._1; val aY = acc._1._2; val aZ = acc._2

        // === PROJECTIVE DOUBLING (y²=x³+7, a=0) ===
        val W = uThree.multiplyMod(aX.multiplyMod(aX, p2), p2)
        val S = aY.multiplyMod(aZ, p2)
        val Bb = aX.multiplyMod(aY, p2).multiplyMod(S, p2)
        val H = W.multiplyMod(W, p2).subtractMod(uEight.multiplyMod(Bb, p2), p2)
        val S2 = S.multiplyMod(S, p2)
        val dX = uTwo.multiplyMod(H, p2).multiplyMod(S, p2)
        val dY = W.multiplyMod(uFour.multiplyMod(Bb, p2).subtractMod(H, p2), p2)
                 .subtractMod(uEight.multiplyMod(aY.multiplyMod(aY, p2), p2).multiplyMod(S2, p2), p2)
        val dZ = uEight.multiplyMod(S2.multiplyMod(S, p2), p2)

        if (b == 0.toByte) {
            ((dX, dY), dZ)
        } else {
            val px = if (b == 3.toByte) g12x else if (b == 2.toByte) g1x else g2x
            val py = if (b == 3.toByte) g12y else if (b == 2.toByte) g1y else g2y

            // === PROJECTIVE ADDITION (Z2=1) ===
            val U1 = py.multiplyMod(dZ, p2)
            val U2 = dY
            val V1 = px.multiplyMod(dZ, p2)
            val V2 = dX
            val Ud = U1.subtractMod(U2, p2)
            val Vd = V1.subtractMod(V2, p2)
            val Vsq = Vd.multiplyMod(Vd, p2)
            val Vcb = Vsq.multiplyMod(Vd, p2)
            val VsqV2 = Vsq.multiplyMod(V2, p2)
            val Wa = dZ
            val UUsqW = Ud.multiplyMod(Ud, p2).multiplyMod(Wa, p2)
            val twoVsqV2 = VsqV2.plusMod(VsqV2, p2)
            val Ar = UUsqW.subtractMod(Vcb.plusMod(twoVsqV2, p2), p2)
            val rX = Vd.multiplyMod(Ar, p2)
            val rY = Ud.multiplyMod(VsqV2.subtractMod(Ar, p2), p2).subtractMod(Vcb.multiplyMod(U2, p2), p2)
            val rZ = Vcb.multiplyMod(Wa, p2)
            ((rX, rY), rZ)
        }
    })

    // Prover-assisted affine conversion
    val z_inv = b2U(zInvBytes)
    val inverseValid = (sum_L1._2.multiplyMod(z_inv, p2) == uOne)
    val e2X = sum_L1._1._1.multiplyMod(z_inv, p2)

    // LEVEL 2 → 3: Native secp256k1 (E1 → Root)
    val l2_sib = b2U(sibBytes.slice(64, 96))
    val s1_L2 = if (dirs(2) == 1.toByte) e2X else l2_sib
    val s2_L2 = if (dirs(2) == 1.toByte) l2_sib else e2X
    val p_L2 = G1_E1.expUnsigned(s1_L2).multiply(G2_E1.expUnsigned(s2_L2))
    val rootX = p_L2.getEncoded.slice(1, 33)

    sigmaProp(bitsValid && inverseValid && (rootX == rootExpected))
}
