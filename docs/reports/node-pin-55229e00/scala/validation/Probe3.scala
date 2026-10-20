//> using scala 2.12.21
//> using dep org.ergoplatform::ergo-core:6.0.8
//> using dep org.ergoplatform::ergo-wallet:6.0.8
//> using dep org.scorexfoundation::sigma-state:6.0.7
//> using repository "https://gitlab.com/api/v4/projects/61211221/packages/maven"
import java.io.File
import scala.util.{Failure, Success, Try}
import com.typesafe.config.{ConfigFactory, ConfigResolveOptions}
import net.ceedubs.ficus.Ficus._
import net.ceedubs.ficus.readers.ArbitraryTypeReader._
import org.ergoplatform.ErgoBox
import org.ergoplatform.mining.groupElemFromBytes
import org.ergoplatform.modifiers.history.CPreHeader
import org.ergoplatform.modifiers.mempool.ErgoTransactionSerializer
import org.ergoplatform.nodeView.state.{UpcomingStateContext, VotingData}
import org.ergoplatform.settings.{ChainSettings, ChainSettingsReader, ErgoValidationSettings, ErgoValidationSettingsUpdate, Parameters}
import org.ergoplatform.wallet.interpreter.ErgoInterpreter
import scorex.crypto.hash.Blake2b256
import scorex.util.bytesToId
import scorex.util.encode.Base16
import sigma.serialization.{SigmaSerializer, ErgoTreeSerializer}

object Probe3 {
  def h(s: String) = Base16.decode(s).get
  def main(args: Array[String]): Unit = {
    import ChainSettingsReader._
    val res = new File(args(0))
    val cfg = ConfigFactory.parseFile(new File(res, "mainnet.conf")).withFallback(ConfigFactory.parseFile(new File(res, "application.conf")))
      .resolve(ConfigResolveOptions.noSystem().setAllowUnresolved(true))
    implicit val chain: ChainSettings = cfg.as[ChainSettings]("ergo.chain")
    def jar(c: Class[_]) = c.getProtectionDomain.getCodeSource.getLocation.getPath.split('/').last
    println(s"versions: ergo-core ${jar(classOf[org.ergoplatform.modifiers.mempool.ErgoTransaction])} ergo-wallet ${jar(classOf[ErgoInterpreter])} sigma ${classOf[sigma.ast.ErgoTree].getProtectionDomain.getCodeSource.getLocation.getPath.split('/').last} java ${System.getProperty("java.runtime.version")}")
    // fixture parameters (canonical-encoding-rejection.fixture), block version 1
    val table = org.ergoplatform.settings.MainnetLaunchParameters.parametersTable ++ Map[Byte, Int](
      Parameters.StorageFeeFactorIncrease -> 1250000, Parameters.MinValuePerByteIncrease -> 360,
      Parameters.MaxBlockSizeIncrease -> 524288, Parameters.MaxBlockCostIncrease -> 8001091,
      Parameters.TokenAccessCostIncrease -> 100, Parameters.InputCostIncrease -> 2000,
      Parameters.DataInputCostIncrease -> 100, Parameters.OutputCostIncrease -> 100,
      Parameters.BlockVersion -> 1)
    val params = new Parameters(100, table, ErgoValidationSettingsUpdate.empty)
    val pre = CPreHeader(1.toByte, bytesToId(h("44" * 32)), 1700000000000L, 117586360L, 100, Array[Byte](0, 0, 0),
      groupElemFromBytes(h("0279be667ef9dcbbac55a06295ce870b07029bfcdb2dce28d959f2815b16f81798")))
    val stateCtx = UpcomingStateContext(Seq.empty, None, pre, chain.genesisStateDigest, params, ErgoValidationSettings.initial, VotingData.empty)(chain)
    implicit val verifier: ErgoInterpreter = ErgoInterpreter(params)
    def box(x: String) = ErgoBox.sigmaSerializer.parse(SigmaSerializer.startReader(h(x)))
    def run(label: String, boxHex: String, txHex: String): Unit = {
      val b = box(boxHex)
      println(s"[$label] input box id ${Base16.encode(b.id)}")
      Try(ErgoTransactionSerializer.parseBytes(h(txHex))) match {
        case Failure(e) => println(s"[$label] PARSE REJECTED: ${e.getClass.getName}: ${e.getMessage}")
        case Success(tx) =>
          val reser = Base16.encode(ErgoTransactionSerializer.toBytes(tx))
          println(s"[$label] parsed; re-serialized equal to input bytes: ${reser == txHex}; tx id ${tx.id}")
          val sl = tx.statelessValidity()
          val full = sl.flatMap(_ => tx.statefulValidity(IndexedSeq(b), IndexedSeq.empty, stateCtx, 0L))
          println(s"[$label] stateless ${sl.isSuccess}${sl.failed.toOption.map(e => " " + e.getMessage).getOrElse("")}")
          println(s"[$label] stateful ACCEPTED=${full.isSuccess} ${full.toOption.map(c => s"cost $c").getOrElse(full.failed.get.getMessage.take(300))}")
      }
    }
    // Item 2: the forge vector's exact bytes (input count 0x81 0x00)
    val box2 = "c0843d10010101d17300640000" + "11" * 32 + "00"
    val tx2 = "8100" + "87ca67e4822b4c8777457f779eb98fb3a348fc40d9a3de8940a124ab29c4ad6a" + "0000000001c0843d10010101d17300640000"
    run("item2 canonical-encoding-rejection", box2, tx2)
    run("item3 create output with tree 10810001d17300", box2, "01" + "87ca67e4822b4c8777457f779eb98fb3a348fc40d9a3de8940a124ab29c4ad6a" + "0000000001c0843d" + "10810001d17300" + "640000")
    // Item 3: the overlong tree 10810001d17300 (constant count 0x81 0x00)
    val tree3 = "10810001d17300"
    Try(ErgoTreeSerializer.DefaultSerializer.deserializeErgoTree(h(tree3))) match {
      case Failure(e) => println(s"[item3] tree PARSE REJECTED: ${e.getClass.getName}: ${e.getMessage}")
      case Success(t) => println(s"[item3] tree parsed: header ${t.header} constants ${t.constants.length} root ${t.root.fold(u => "UNPARSED " + u, p => p.toString)}; re-serialized ${Base16.encode(ErgoTreeSerializer.DefaultSerializer.serializeErgoTree(t))}; bytes field equal ${Base16.encode(t.bytes) == tree3}")
    }
    val box3 = "c0843d" + tree3 + "640000" + "11" * 32 + "00"
    val b3 = box(box3)
    println(s"[item3] box parsed; tree bytes in box ${Base16.encode(b3.ergoTree.bytes)}; re-serialized box equal ${Base16.encode(b3.bytes) == box3}")
    val tx3 = "01" + Base16.encode(b3.id) + "0000000001c0843d10010101d17300640000"
    run("item3 overlong tree spend", box3, tx3)
  }
}
