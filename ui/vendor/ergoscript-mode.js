// ErgoScript syntax for CodeMirror 5 (simple-mode). Scala-flavoured: keywords,
// context objects, types, `$name` compile-time constants, comments, strings.
(function () {
  "use strict";
  const keywords = /(?:val|def|if|else|true|false|match|case|new|lazy|type|import)\b/;
  const context = /(?:HEIGHT|SELF|INPUTS|OUTPUTS|CONTEXT|MinerPubkey|LastBlockUtxoRootHash|Global|dataInputs|getVar|sigmaProp|proveDlog|proveDHTuple|atLeast|allOf|anyOf|blake2b256|sha256|fromBase16|fromBase58|fromBase64|decodePoint|PK|min|max|byteArrayToBigInt|byteArrayToLong|longToByteArray|xorOf|substConstants|deserialize|executeFromVar|executeFromSelfReg|groupGenerator)\b/;
  const types = /(?:Boolean|Byte|Short|Int|Long|BigInt|UnsignedBigInt|GroupElement|SigmaProp|Box|AvlTree|Coll|Option|Header|PreHeader|Context|Any|Unit|String)\b/;
  CodeMirror.defineSimpleMode("ergoscript", {
    start: [
      { regex: /\/\*\*/, token: "comment", next: "doc" },
      { regex: /\/\*/, token: "comment", next: "comment" },
      { regex: /\/\/.*/, token: "comment" },
      { regex: /"(?:[^\\"]|\\.)*"?/, token: "string" },
      { regex: /@contract\b/, token: "meta" },
      { regex: /\$[A-Za-z_][A-Za-z0-9_]*/, token: "variable-3" },
      { regex: keywords, token: "keyword" },
      { regex: context, token: "builtin" },
      { regex: types, token: "type" },
      { regex: /0x[0-9a-fA-F]+|\d+[LlyY]?/, token: "number" },
      { regex: /[-+\/*=<>!&|^%]+/, token: "operator" },
      { regex: /[{[(]/, indent: true },
      { regex: /[}\])]/, dedent: true },
      { regex: /[A-Za-z_][A-Za-z0-9_]*/, token: "variable" },
    ],
    doc: [
      { regex: /.*?\*\//, token: "comment", next: "start" },
      { regex: /@param\b/, token: "meta" },
      { regex: /.*/, token: "comment" },
    ],
    comment: [
      { regex: /.*?\*\//, token: "comment", next: "start" },
      { regex: /.*/, token: "comment" },
    ],
    meta: { lineComment: "//", blockCommentStart: "/*", blockCommentEnd: "*/", dontIndentStates: ["comment", "doc"] },
  });
})();
