"""Method sweep: every method of SBox / SContext / SHeader / SPreHeader /
SGlobal / SAvlTree / SGroupElement / SSigmaProp / SOption / SCollection /
numerics, exercised once against a fully specified scenario with expected
values computed here, independently. Two suites: tree version 2 (v5-era
methods) and 3 (v6). A case that errors or evaluates false is a finding
about the node's evaluator or compiler. Run from the repo root after
`cargo build -p ergo-sandbox`."""
import json, os, subprocess, hashlib
E = os.path.expanduser("~/.cache/cargo-target/debug/ergo-es")
point = lambda x, base=None: subprocess.check_output([E, "point", "%064x" % x] + (["--base", base] if base else []), text=True).strip()
G = point(1); G2 = point(2); G3 = point(3)
X = "11" * 32          # SELF box id
TOK1 = "aa" * 32; TOK2 = "bb" * 32
H0 = {"id": "01" * 32, "version": 3, "parentId": "02" * 32, "adProofsRoot": "03" * 32, "stateRoot": "04" * 32 + "09",
      "transactionsRoot": "05" * 32, "timestamp": 1_700_000_120_000, "nBits": 0x01234567, "height": 999,
      "extensionRoot": "06" * 32, "minerPk": G2, "powOnetimePk": G3, "powNonce": "0708090a0b0c0d0e", "powDistance": "123456789", "votes": [1, 2, 3]}
H1 = {"height": 998, "timestamp": 1_700_000_000_000, "id": "0a" * 32}
PRE = {"version": 3, "parentId": "0b" * 32, "timestamp": 1_700_000_240_000, "nBits": 0x0189abcd, "votes": [4, 5, 6]}
def base_scenario():
    return {
        "height": 1000,
        "selfBox": {"value": 5_000_000_000, "boxId": X, "creationHeight": 900,
                    "tokens": [{"id": TOK1, "amount": 7}, {"id": TOK2, "amount": 11}],
                    "registers": {"R4": {"type": "Int", "value": 42}, "R5": {"type": "Long", "value": 43}, "R6": {"type": "Coll[Byte]", "value": "cafe"},
                                  "R7": {"type": "Boolean", "value": True}, "R8": {"type": "GroupElement", "value": G}, "R9": {"type": "SigmaProp", "value": G2}}},
        "inputs": [{"value": 1_000_000, "boxId": "22" * 32, "ergoTree": "10010101d17300", "tokens": [{"id": TOK1, "amount": 1}]}],
        "outputs": [{"value": 4_000_000_000, "ergoTree": "$self", "creationHeight": 1000, "tokens": [{"id": TOK1, "amount": 7}]},
                    {"value": 1_000_000, "ergoTree": "0008cd" + G2}],
        "dataInputs": [{"value": 777, "ergoTree": "10010101d17300", "registers": {"R4": {"type": "Long", "value": 555}}}],
        "headers": [H0, H1], "preHeader": PRE, "minerPubkey": G3,
        "contextVars": {"0": {"type": "Int", "value": 17}, "1": {"type": "Coll[Byte]", "value": "0102030405"}, "2": {"type": "Long", "value": 99}},
        "avl": {"t": {"keyLength": 32, "entries": [["33" * 32, "0a0b"], ["44" * 32, "0c"]], "operations": [{"lookup": {"key": "33" * 32}}, {"lookup": {"key": "44" * 32}}]},
                "one": {"keyLength": 32, "entries": [["33" * 32, "0a0b"], ["44" * 32, "0c"]], "operations": [{"lookup": {"key": "33" * 32}}]},
                "two": {"keyLength": 32, "entries": [["33" * 32, "0a0b"], ["44" * 32, "0c"]], "operations": [{"lookup": {"key": "44" * 32}}]},
                "none": {"keyLength": 32, "entries": [["33" * 32, "0a0b"]], "operations": [{"lookup": {"key": "66" * 32}}]},
                "ins": {"keyLength": 32, "entries": [["33" * 32, "0a0b"]], "operations": [{"insert": {"key": "55" * 32, "value": "0d"}}]},
                "upd": {"keyLength": 32, "entries": [["33" * 32, "0a0b"]], "operations": [{"update": {"key": "33" * 32, "value": "0e0f"}}]},
                "rem": {"keyLength": 32, "entries": [["33" * 32, "0a0b"], ["44" * 32, "0c"]], "operations": [{"remove": {"key": "44" * 32}}]},
                "iou": {"keyLength": 32, "entries": [["33" * 32, "0a0b"]], "operations": [{"insertOrUpdate": {"key": "33" * 32, "value": "0e"}}]}},
    }
def hexs(s): return f'fromBase16("{s}")'
b2 = lambda b: hashlib.blake2b(b, digest_size=32).hexdigest()
s256 = lambda b: hashlib.sha256(b).hexdigest()
# (name, boolean ErgoScript expression, min tree version, extra scenario overrides)
CASES = [
 # ── SBox
 ("Box.value", "SELF.value == 5000000000L", 0, {}),
 ("Box.propositionBytes", "OUTPUTS(0).propositionBytes == SELF.propositionBytes", 0, {}),
 ("Box.id is blake2b256(bytes) (a box whose id the scenario did not name)", "blake2b256(OUTPUTS(1).bytes) == OUTPUTS(1).id", 0, {}),
 ("Box.id given", f"SELF.id == {hexs(X)}", 0, {}),
 ("Box.bytesWithoutRef is bytes minus the ref (32-byte tx id + VLQ index 0)", "SELF.bytes.size == SELF.bytesWithoutRef.size + 33", 0, {}),
 ("Box.creationInfo._1", "SELF.creationInfo._1 == 900", 0, {}),
 ("Box.creationInfo._2 is 34 bytes", "SELF.creationInfo._2.size == 34", 0, {}),
 ("Box.tokens", f"SELF.tokens.size == 2 && SELF.tokens(0)._1 == {hexs(TOK1)} && SELF.tokens(1)._2 == 11L", 0, {}),
 ("Box.R4[Int]", "SELF.R4[Int].get == 42", 0, {}),
 ("Box.R5[Long]", "SELF.R5[Long].get == 43L", 0, {}),
 ("Box.R6[Coll[Byte]]", 'SELF.R6[Coll[Byte]].get == fromBase16("cafe")', 0, {}),
 ("Box.R7[Boolean]", "SELF.R7[Boolean].get", 0, {}),
 ("Box.R8[GroupElement]", f"SELF.R8[GroupElement].get == decodePoint({hexs(G)})", 0, {}),
 ("Box.R9[SigmaProp]", f"SELF.R9[SigmaProp].get.propBytes == {hexs('0008cd' + G2)}", 0, {}),
 ("Box.R0 is value", "SELF.R0[Long].get == SELF.value", 0, {}),
 ("Box.R1 is propositionBytes", "SELF.R1[Coll[Byte]].get == SELF.propositionBytes", 0, {}),
 ("Box.R2 is tokens", "SELF.R2[Coll[(Coll[Byte], Long)]].get == SELF.tokens", 0, {}),
 ("Box.R3 is creationInfo", "SELF.R3[(Int, Coll[Byte])].get == SELF.creationInfo", 0, {}),
 ("Box register absent is None", "!(OUTPUTS(1).R4[Int].isDefined) && OUTPUTS(1).R4[Int].getOrElse(5) == 5", 0, {}),
 ("Box.getReg (v6)", "SELF.getReg[Int](4).get == 42", 3, {}),
 # ── SContext
 ("Context.dataInputs", "CONTEXT.dataInputs.size == 1 && CONTEXT.dataInputs(0).value == 777L && CONTEXT.dataInputs(0).R4[Long].get == 555L", 0, {}),
 ("Context.headers", "CONTEXT.headers.size == 2 && CONTEXT.headers(1).height == 998", 0, {}),
 ("Context.preHeader.height is HEIGHT", "CONTEXT.preHeader.height == HEIGHT", 0, {}),
 ("Context.INPUTS / OUTPUTS", "INPUTS.size == 2 && OUTPUTS.size == 2 && INPUTS(1).value == 1000000L", 0, {}),
 ("Context.HEIGHT", "HEIGHT == 1000", 0, {}),
 ("Context.SELF is INPUTS(0)", "SELF.id == INPUTS(0).id", 0, {}),
 ("Context.selfBoxIndex", "CONTEXT.selfBoxIndex == 0", 0, {}),
 ("Context.LastBlockUtxoRootHash", "CONTEXT.LastBlockUtxoRootHash.digest == CONTEXT.headers(0).stateRoot.digest", 0, {}),
 ("Context.minerPubKey", f"CONTEXT.minerPubKey == {hexs(G3)}", 0, {}),
 ("Context.getVar", 'getVar[Int](0).get == 17 && getVar[Coll[Byte]](1).get == fromBase16("0102030405") && !(getVar[Int](5).isDefined)', 0, {}),
 ("Context.getVarFromInput (v6)", "CONTEXT.getVarFromInput[Int](0, 0).get == 17", 3, {}),
 # ── SHeader
 ("Header.id", f"CONTEXT.headers(0).id == {hexs(H0['id'])}", 0, {}),
 ("Header.version", "CONTEXT.headers(0).version == 3.toByte", 0, {}),
 ("Header.parentId", f"CONTEXT.headers(0).parentId == {hexs(H0['parentId'])}", 0, {}),
 ("Header.ADProofsRoot", f"CONTEXT.headers(0).ADProofsRoot == {hexs(H0['adProofsRoot'])}", 0, {}),
 ("Header.stateRoot", f"CONTEXT.headers(0).stateRoot.digest == {hexs(H0['stateRoot'])}", 0, {}),
 ("Header.transactionsRoot", f"CONTEXT.headers(0).transactionsRoot == {hexs(H0['transactionsRoot'])}", 0, {}),
 ("Header.timestamp", f"CONTEXT.headers(0).timestamp == {H0['timestamp']}L", 0, {}),
 ("Header.nBits", f"CONTEXT.headers(0).nBits == {H0['nBits']}L", 0, {}),
 ("Header.height", "CONTEXT.headers(0).height == 999", 0, {}),
 ("Header.extensionRoot", f"CONTEXT.headers(0).extensionRoot == {hexs(H0['extensionRoot'])}", 0, {}),
 ("Header.minerPk", f"CONTEXT.headers(0).minerPk == decodePoint({hexs(G2)})", 0, {}),
 ("Header.powOnetimePk", f"CONTEXT.headers(0).powOnetimePk == decodePoint({hexs(G3)})", 0, {}),
 ("Header.powNonce", f"CONTEXT.headers(0).powNonce == {hexs(H0['powNonce'])}", 0, {}),
 ("Header.powDistance", "CONTEXT.headers(0).powDistance == 123456789L.toBigInt", 0, {}),
 ("Header.votes", "CONTEXT.headers(0).votes == Coll(1.toByte, 2.toByte, 3.toByte)", 0, {}),
 # ── SPreHeader
 ("PreHeader.version", "CONTEXT.preHeader.version == 3.toByte", 0, {}),
 ("PreHeader.parentId", f"CONTEXT.preHeader.parentId == {hexs(PRE['parentId'])}", 0, {}),
 ("PreHeader.timestamp", f"CONTEXT.preHeader.timestamp == {PRE['timestamp']}L", 0, {}),
 ("PreHeader.nBits", f"CONTEXT.preHeader.nBits == {PRE['nBits']}L", 0, {}),
 ("PreHeader.minerPk is the miner key", f"CONTEXT.preHeader.minerPk == decodePoint({hexs(G3)})", 0, {}),
 ("PreHeader.votes", "CONTEXT.preHeader.votes == Coll(4.toByte, 5.toByte, 6.toByte)", 0, {}),
 # ── SGlobal / global functions
 ("groupGenerator", f"groupGenerator == decodePoint({hexs(G)})", 0, {}),
 ("xor", 'xor(fromBase16("ff00"), fromBase16("0ff0")) == fromBase16("f0f0")', 0, {}),
 ("blake2b256", f'blake2b256(fromBase16("0102")) == {hexs(b2(bytes.fromhex("0102")))}', 0, {}),
 ("sha256", f'sha256(fromBase16("0102")) == {hexs(s256(bytes.fromhex("0102")))}', 0, {}),
 ("longToByteArray / byteArrayToLong", 'byteArrayToLong(longToByteArray(-2L)) == -2L && longToByteArray(1L) == fromBase16("0000000000000001")', 0, {}),
 ("byteArrayToBigInt", 'byteArrayToBigInt(fromBase16("0100")) == 256L.toBigInt', 0, {}),
 ("xorOf / allOf / anyOf", "xorOf(Coll(true, false)) && allOf(Coll(true, true)) && anyOf(Coll(false, true))", 0, {}),
 ("min / max", "min(3, 4) == 3 && max(3L, 4L) == 4L", 0, {}),
 ("Global.serialize (v6)", 'Global.serialize(1) == fromBase16("02")', 3, {}),
 ("Global.fromBigEndianBytes (v6)", 'Global.fromBigEndianBytes[Long](fromBase16("0000000000000100")) == 256L', 3, {}),
 ("Global.encodeNbits / decodeNbits (v6)", "Global.decodeNbits(Global.encodeNbits(1000L.toBigInt)) == 1000L.toBigInt", 3, {}),
 ("Global.some / none (v6)", "Global.some[Int](1).isDefined && !(Global.none[Int]().isDefined)", 3, {}),
 ("Global.deserializeTo (v6)", 'Global.deserializeTo[Int](fromBase16("02")) == 1', 3, {}),
 # ── SGroupElement
 ("GroupElement.getEncoded", f"groupGenerator.getEncoded == {hexs(G)}", 0, {}),
 ("GroupElement.exp", f"groupGenerator.exp(2L.toBigInt) == decodePoint({hexs(G2)})", 0, {}),
 ("GroupElement.multiply", f"groupGenerator.multiply(groupGenerator) == decodePoint({hexs(G2)})", 0, {}),
 ("GroupElement.negate", f"groupGenerator.negate.getEncoded == {hexs('03' + G[2:])}", 0, {}),
 ("GroupElement.expUnsigned (v6)", f"groupGenerator.expUnsigned(2L.toBigInt.toUnsigned) == decodePoint({hexs(G2)})", 3, {}),
 # ── SSigmaProp
 ("SigmaProp.propBytes", f"proveDlog(decodePoint({hexs(G2)})).propBytes == {hexs('0008cd' + G2)}", 0, {}),
 # ── SOption
 ("Option.isDefined / get / getOrElse", "SELF.R4[Int].isDefined && SELF.R4[Int].get == 42 && OUTPUTS(1).R4[Long].getOrElse(7L) == 7L", 0, {}),
 ("Option.map", "SELF.R4[Int].map({ (x: Int) => x + 1 }).get == 43", 0, {}),
 ("Option.filter", "!(SELF.R4[Int].filter({ (x: Int) => x > 100 }).isDefined)", 0, {}),
 # ── SCollection
 ("Coll.size / apply / getOrElse", "Coll(1, 2, 3).size == 3 && Coll(1, 2, 3)(1) == 2 && Coll(1, 2, 3).getOrElse(9, 0) == 0", 0, {}),
 ("Coll.map / exists / forall", "Coll(1, 2).map({ (x: Int) => x * 2 }) == Coll(2, 4) && Coll(1, 2).exists({ (x: Int) => x == 2 }) && Coll(1, 2).forall({ (x: Int) => x > 0 })", 0, {}),
 ("Coll.fold", "Coll(1, 2, 3).fold(0, { (a: Int, b: Int) => a + b }) == 6", 0, {}),
 ("Coll.slice / filter / append", "Coll(1, 2, 3, 4).slice(1, 3) == Coll(2, 3) && Coll(1, 2, 3).filter({ (x: Int) => x % 2 == 1 }) == Coll(1, 3) && Coll(1).append(Coll(2)) == Coll(1, 2)", 0, {}),
 ("Coll.indices", "Coll(5, 6).indices == Coll(0, 1)", 0, {}),
 ("Coll.flatMap", "Coll(1, 2).flatMap({ (x: Int) => Coll(x, x) }) == Coll(1, 1, 2, 2)", 0, {}),
 ("Coll.patch / updated / updateMany", "Coll(1, 2, 3).patch(1, Coll(9), 1) == Coll(1, 9, 3) && Coll(1, 2).updated(0, 7) == Coll(7, 2) && Coll(1, 2, 3).updateMany(Coll(0, 2), Coll(8, 9)) == Coll(8, 2, 9)", 0, {}),
 ("Coll.indexOf", "Coll(4, 5, 6).indexOf(5, 0) == 1 && Coll(4, 5, 6).indexOf(7, 0) == -1", 0, {}),
 ("Coll.zip", "Coll(1, 2).zip(Coll(3, 4)) == Coll((1, 3), (2, 4))", 0, {}),
 ("Coll.reverse (v6)", "Coll(1, 2, 3).reverse == Coll(3, 2, 1)", 3, {}),
 ("Coll.startsWith / endsWith (v6)", "Coll(1, 2, 3).startsWith(Coll(1, 2)) && Coll(1, 2, 3).endsWith(Coll(3))", 3, {}),
 ("Coll.get (v6)", "{ val c = Coll(1, 2); c.get(1).get == 2 && !(c.get(5).isDefined) }", 3, {}),
 ("Tuple.size / apply", "(1, 2L)._1 == 1 && (1, 2L)._2 == 2L", 0, {}),
 # ── numerics
 ("numeric casts", "300.toLong == 300L && 300L.toInt == 300 && 3.toByte == 3.toByte && 7.toShort.toInt == 7 && 5L.toBigInt == 5.toBigInt", 0, {}),
 ("arithmetic", "7 / 2 == 3 && -7 / 2 == -3 && 7 % 3 == 1 && -7 % 3 == -1 && 2L * 3L == 6L", 0, {}),
 ("comparison", "1 < 2 && 2L >= 2L && 3.toBigInt > 2.toBigInt && 1.toByte != 2.toByte", 0, {}),
 ("BigInt arithmetic", "(10.toBigInt * 10.toBigInt) == 100.toBigInt && (7.toBigInt / 2.toBigInt) == 3.toBigInt && (7.toBigInt % 2.toBigInt) == 1.toBigInt", 0, {}),
 ("numeric toBytes / toBits (v6)", '1.toBytes == fromBase16("00000001") && 1.toByte.toBits == Coll(false, false, false, false, false, false, false, true)', 3, {}),
 ("BigInt to unsigned (v6)", "5.toBigInt.toUnsigned.toSigned == 5.toBigInt", 3, {}),
 # ── SAvlTree
 ("AvlTree.digest / keyLength / valueLengthOpt / flags", "SELF.R4[AvlTree].get.digest.size == 33 && SELF.R4[AvlTree].get.keyLength == 32 && !(SELF.R4[AvlTree].get.valueLengthOpt.isDefined) && SELF.R4[AvlTree].get.isInsertAllowed && SELF.R4[AvlTree].get.isUpdateAllowed && SELF.R4[AvlTree].get.isRemoveAllowed && SELF.R4[AvlTree].get.enabledOperations == 7.toByte", 0,
  {"selfBox": {"value": 1, "registers": {"R4": {"type": "AvlTree", "value": "@avl.t"}}}}),
 ("AvlTree.updateOperations", "SELF.R4[AvlTree].get.updateOperations(1.toByte).enabledOperations == 1.toByte && !(SELF.R4[AvlTree].get.updateOperations(1.toByte).isUpdateAllowed)", 0,
  {"selfBox": {"value": 1, "registers": {"R4": {"type": "AvlTree", "value": "@avl.t"}}}}),
 ("AvlTree.contains", f'SELF.R4[AvlTree].get.contains({hexs("33"*32)}, getVar[Coll[Byte]](0).get)', 0,
  {"selfBox": {"value": 1, "registers": {"R4": {"type": "AvlTree", "value": "@avl.one"}}}, "contextVars": {"0": {"type": "Coll[Byte]", "value": "@avl.one.proof"}}}),
 ("AvlTree.get", f'SELF.R4[AvlTree].get.get({hexs("44"*32)}, getVar[Coll[Byte]](0).get).get == fromBase16("0c")', 0,
  {"selfBox": {"value": 1, "registers": {"R4": {"type": "AvlTree", "value": "@avl.two"}}}, "contextVars": {"0": {"type": "Coll[Byte]", "value": "@avl.two.proof"}}}),
 ("AvlTree.get absent key is None", f'!(SELF.R4[AvlTree].get.get({hexs("66"*32)}, getVar[Coll[Byte]](0).get).isDefined)', 0,
  {"selfBox": {"value": 1, "registers": {"R4": {"type": "AvlTree", "value": "@avl.none"}}}, "contextVars": {"0": {"type": "Coll[Byte]", "value": "@avl.none.proof"}}}),
 ("AvlTree.getMany", f'SELF.R4[AvlTree].get.getMany(Coll({hexs("33"*32)}, {hexs("44"*32)}), getVar[Coll[Byte]](0).get).size == 2', 0,
  {"selfBox": {"value": 1, "registers": {"R4": {"type": "AvlTree", "value": "@avl.t"}}}, "contextVars": {"0": {"type": "Coll[Byte]", "value": "@avl.t.proof"}}}),
 ("AvlTree.insert", f'SELF.R4[AvlTree].get.insert(Coll(({hexs("55"*32)}, fromBase16("0d"))), getVar[Coll[Byte]](0).get).get.digest == getVar[Coll[Byte]](1).get', 0,
  {"selfBox": {"value": 1, "registers": {"R4": {"type": "AvlTree", "value": "@avl.ins"}}}, "contextVars": {"0": {"type": "Coll[Byte]", "value": "@avl.ins.proof"}, "1": {"type": "Coll[Byte]", "value": "@avl.ins.digestAfter"}}}),
 ("AvlTree.update", f'SELF.R4[AvlTree].get.update(Coll(({hexs("33"*32)}, fromBase16("0e0f"))), getVar[Coll[Byte]](0).get).get.digest == getVar[Coll[Byte]](1).get', 0,
  {"selfBox": {"value": 1, "registers": {"R4": {"type": "AvlTree", "value": "@avl.upd"}}}, "contextVars": {"0": {"type": "Coll[Byte]", "value": "@avl.upd.proof"}, "1": {"type": "Coll[Byte]", "value": "@avl.upd.digestAfter"}}}),
 ("AvlTree.remove", f'SELF.R4[AvlTree].get.remove(Coll({hexs("44"*32)}), getVar[Coll[Byte]](0).get).get.digest == getVar[Coll[Byte]](1).get', 0,
  {"selfBox": {"value": 1, "registers": {"R4": {"type": "AvlTree", "value": "@avl.rem"}}}, "contextVars": {"0": {"type": "Coll[Byte]", "value": "@avl.rem.proof"}, "1": {"type": "Coll[Byte]", "value": "@avl.rem.digestAfter"}}}),
 ("AvlTree.updateDigest", "SELF.R4[AvlTree].get.updateDigest(getVar[Coll[Byte]](1).get).digest == getVar[Coll[Byte]](1).get", 0,
  {"selfBox": {"value": 1, "registers": {"R4": {"type": "AvlTree", "value": "@avl.ins"}}}, "contextVars": {"1": {"type": "Coll[Byte]", "value": "@avl.ins.digestAfter"}}}),
 ("AvlTree.insertOrUpdate (v6)", f'SELF.R4[AvlTree].get.insertOrUpdate(Coll(({hexs("33"*32)}, fromBase16("0e"))), getVar[Coll[Byte]](0).get).get.digest == getVar[Coll[Byte]](1).get', 3,
  {"selfBox": {"value": 1, "registers": {"R4": {"type": "AvlTree", "value": "@avl.iou"}}}, "contextVars": {"0": {"type": "Coll[Byte]", "value": "@avl.iou.proof"}, "1": {"type": "Coll[Byte]", "value": "@avl.iou.digestAfter"}}}),
 # ── sigma
 ("proveDlog / proveDHTuple reduce to needsProof shapes", f"sigmaProp(true) && proveDlog(decodePoint({hexs(G2)})).propBytes.size > 0", 0, {}),
 ("atLeast", f"atLeast(1, Coll(sigmaProp(true), proveDlog(decodePoint({hexs(G2)})))).propBytes.size > 0", 0, {}),
 ("substConstants", 'substConstants(fromBase16("100104c801d191a37300"), Coll(0), Coll(300)).size > 0', 0, {}),
]
def case(name, expr, tv, over):
    sc = base_scenario()
    for k, v in over.items(): sc[k] = v
    sc["name"] = name; sc["expect"] = "pass"; sc["source"] = f"sigmaProp({expr})"
    return sc
# One file, every case with its own source and tree version; the sweep test
# (ergo-sandbox/tests/method_sweep.rs) runs each through the suite runner.
out = []
for tv in (2, 3):
    for (n, e, mv, o) in CASES:
        if mv <= tv:
            c = case(n, e, mv, o)
            out.append({"name": f"{n} [tree v{tv}]", "source": c.pop("source"), "treeVersion": tv, "scenario": c})
json.dump(out, open("examples/tests/method-sweep.json", "w"), indent=1)
print("method-sweep.json:", len(out), "cases")
