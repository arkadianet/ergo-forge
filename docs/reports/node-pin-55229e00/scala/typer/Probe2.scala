//> using scala 2.12.21
//> using dep org.scorexfoundation::sigma-state:6.0.7
//> using dep io.circe::circe-parser:0.14.15
import io.circe.parser._
import io.circe.Json
import sigma.VersionContext
import sigma.Colls
import sigma.ast._
import sigma.ast.syntax._
import sigma.data.{CSigmaProp, ProveDlog, CGroupElement}
import sigma.compiler.{SigmaCompiler, CompilerSettings}
import sigma.compiler.ir.CompiletimeIRContext
import sigma.serialization.{ErgoTreeSerializer, GroupElementSerializer, SigmaSerializer}
import scorex.util.encode.Base16

object Probe2 {
  def ge(h: String) = GroupElementSerializer.parse(SigmaSerializer.startReader(Base16.decode(h).get))
  def value(t: String, v: Json): Any = t match {
    case "Int" => v.asNumber.get.toInt.get
    case "Long" => v.asNumber.get.toLong.get
    case "Coll[Byte]" => Colls.fromArray(Base16.decode(v.asString.get).get)
    case "SigmaProp" => CSigmaProp(ProveDlog(ge(v.asString.get)))
    case "GroupElement" => CGroupElement(ge(v.asString.get))
  }
  def main(args: Array[String]): Unit = {
    val cases = parse(scala.io.Source.fromFile(args(0)).mkString).toOption.get.asArray.get
    for (c <- cases) {
      val id = c.hcursor.get[String]("id").toOption.get
      val ctrl = c.hcursor.get[Boolean]("control").toOption.get
      val src = c.hcursor.get[String]("source").toOption.get
      val env: Map[String, Any] = c.hcursor.downField("env").focus.get.asArray.get.map { e =>
        val h = e.hcursor
        h.get[String]("name").toOption.get -> value(h.get[String]("type").toOption.get, h.downField("value").focus.get)
      }.toMap
      val rustTree = c.hcursor.get[String]("tree").toOption
      val res = try {
        VersionContext.withVersions(3, 3) {
          implicit val IR: CompiletimeIRContext = new CompiletimeIRContext
          val comp = SigmaCompiler(CompilerSettings(sys.env.getOrElse("NETPREFIX","0").toByte, TransformingSigmaBuilder, lowerMethodCalls = true))
          val prop = comp.compile(env, src).buildTree.asSigmaProp
          val hdr = rustTree.map(t => Base16.decode(t).get(0)).getOrElse(0x13.toByte)
          val tree = ErgoTree.withSegregation(ErgoTree.HeaderType @@ hdr, prop)
          val hex = Base16.encode(ErgoTreeSerializer.DefaultSerializer.serializeErgoTree(tree))
          "compiled" + (rustTree match { case Some(t) => if (t == hex) " tree=rust" else " tree=DIFF" ; case None => "" })
        }
      } catch { case e: Throwable => "ERROR " + e.getClass.getSimpleName + ": " + String.valueOf(e.getMessage).take(160).replace('\n',' ') }
      println(s"${if (ctrl) "control" else "changed"} | $id | $res")
    }
  }
}
