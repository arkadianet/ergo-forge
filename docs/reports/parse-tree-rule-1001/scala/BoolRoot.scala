//> using scala 2.12.21
//> using dep org.scorexfoundation::sigma-state:6.0.7
//> using repository "https://gitlab.com/api/v4/projects/61211221/packages/maven"
// Run: scala-cli run BoolRoot.scala -- <ErgoTree hex>...
import scala.util.{Failure, Success, Try}
import org.ergoplatform._
import org.ergoplatform.validation.ValidationRules
import scorex.util.encode.Base16
import sigma.{Colls, VersionContext}
import sigma.ast.ErgoTree
import sigma.crypto.CryptoConstants
import sigma.data.{AvlTreeData, CSigmaDslBuilder}
import sigma.interpreter.{ContextExtension, ProverResult}
import sigma.serialization.{ErgoTreeSerializer, GroupElementSerializer, SigmaSerializer}
import sigma.util.Extensions.EcpOps
import sigmastate.eval.CPreHeader
import sigmastate.interpreter.CErgoTreeEvaluator.DefaultEvalSettings

// sigma-state 6.0.7: ErgoTrees whose root is a Boolean constant placeholder (not a SigmaProp).
// 1) deserialize as a box's script is deserialized; 2) where that yields a tree, spend a box
// protected by it with an empty proof at height 100 (ErgoLikeInterpreter.verify).
object BoolRoot {
  val pk = GroupElementSerializer.toBytes(CryptoConstants.dlogGroup.generator)
  def spend(tree: ErgoTree) = {
    val self = new ErgoBoxCandidate(1000000L, tree, 0).toBox(ErgoBox.allZerosModifierId, 0)
    val pre = CPreHeader(0, Colls.emptyColl[Byte], 3, 0, 100,
      GroupElementSerializer.parse(SigmaSerializer.startReader(pk)).toGroupElement, Colls.emptyColl[Byte])
    val ctx = new ErgoLikeContext(AvlTreeData.dummy, CSigmaDslBuilder.Colls.emptyColl, pre, IndexedSeq(),
      IndexedSeq(self), ErgoLikeTransaction(IndexedSeq(), IndexedSeq()), 0, ContextExtension.empty,
      ValidationRules.currentSettings, DefaultEvalSettings.scriptCostLimitInEvaluator, 0L, 3.toByte)
    val interp = new ErgoLikeInterpreter { override type CTX = ErgoLikeContext }
    interp.verify(tree, ctx, ProverResult.empty, Array[Byte](1, 2, 3))
  }
  def main(args: Array[String]): Unit = {
    def jar(c: Class[_]) = c.getProtectionDomain.getCodeSource.getLocation.getPath.split('/').last
    println(s"versions: sigma ${jar(classOf[ErgoTree])} java ${System.getProperty("java.runtime.version")}")
    for (hex <- args) run(hex)
  }
  def run(hex: String): Unit = {
    val b = Base16.decode(hex).get
    Try(VersionContext.withVersions(3.toByte, 0.toByte)(ErgoTreeSerializer.DefaultSerializer.deserializeErgoTree(b))) match {
      case Failure(e) => println(s"$hex: box script REJECTED at deserialization: $e")
      case Success(tree) =>
        val kind = if (tree.root.isLeft) "unparsed (Left)" else s"parsed, root ${tree.root.right.get}"
        val v = Try(VersionContext.withVersions(3.toByte, 0.toByte)(spend(tree))).flatten
        println(s"$hex: deserialized: $kind; spend with empty proof: " +
          v.fold(e => s"REJECTED: ${e.getMessage.take(140)}", r => s"verified=${r._1}"))
    }
  }
}
