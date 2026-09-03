//! Sandbox integration tests — the eval engine against the consensus
//! primitives, from a downstream caller's view.
//!
//! Section dividers per CLAUDE.md: helpers / happy path / verdicts /
//! error paths / cost / inspect.

use ergo_sandbox::{eval_scenario, inspect, Scenario, Verdict};

// ----- helpers -----

fn eval_json(json: &str) -> ergo_sandbox::EvalOutcome {
    let sc: Scenario = serde_json::from_str(json).expect("scenario JSON");
    eval_scenario(&sc).expect("eval")
}

/// The seed-vector tree for `sigmaProp(HEIGHT > 100)`
/// (`test-vectors/ergoscript/compile/compile_seed.json`).
const HEIGHT_TREE: &str = "100104c801d191a37300";

const TEST_PK_BASE58: &str = "9fSgJ7BmUxBQJ454prQDQ7fQMBkXPLaAmDnimgTtjym6FYPHjAV";

// ----- happy path / verdicts -----

#[test]
fn height_guard_passes_and_fails_on_scenario_height() {
    let pass = eval_json(r#"{"source":"sigmaProp(HEIGHT > 100)","height":200}"#);
    assert_eq!(pass.verdict, Verdict::Pass);
    assert_eq!(pass.reduced_to.as_deref(), Some("true"));
    assert!(pass.cost > 0);

    let fail = eval_json(r#"{"source":"sigmaProp(HEIGHT > 100)","height":99}"#);
    assert_eq!(fail.verdict, Verdict::Fail);
    assert_eq!(fail.reduced_to.as_deref(), Some("false"));
    assert!(fail.error.is_none());
}

#[test]
fn tree_hex_mode_matches_source_mode() {
    let from_hex = eval_json(&format!(r#"{{"tree":"{HEIGHT_TREE}","height":300}}"#));
    let from_source = eval_json(r#"{"source":"sigmaProp(HEIGHT > 100)","height":300}"#);
    assert_eq!(from_hex.verdict, Verdict::Pass);
    assert_eq!(from_hex.tree_hex, from_source.tree_hex);
    assert_eq!(from_hex.cost, from_source.cost);
}

#[test]
fn needs_proof_reports_the_sigma_proposition() {
    let out = eval_json(&format!(
        r#"{{"source":"sigmaProp(HEIGHT > 1000 && PK(\"{TEST_PK_BASE58}\"))","height":2000}}"#
    ));
    assert_eq!(out.verdict, Verdict::NeedsProof);
    // AND collapsed its trivial-true child down to the DLOG proposition.
    let red = out.reduced_to.expect("reducedTo");
    assert!(red.starts_with("ProveDlog("), "reducedTo was {red}");
    // The trace shows the sigma-protocol children that were reduced.
    assert!(out.trace.iter().any(|t| t.label.contains("SigmaAnd")));
}

#[test]
fn self_box_defaults_to_the_evaluated_tree() {
    // SELF.minStorage? simplest self-reference that needs no other boxes:
    // SELF.value == 0 (the synthetic default).
    let out = eval_json(r#"{"source":"sigmaProp(SELF.value == 0L)","height":100}"#);
    assert_eq!(out.verdict, Verdict::Pass);

    let no = eval_json(r#"{"source":"sigmaProp(SELF.value == 5L)","height":100}"#);
    assert_eq!(no.verdict, Verdict::Fail);
}

#[test]
fn outputs_registers_and_tokens_are_visible_to_the_script() {
    let json = r#"{
        "source": "sigmaProp(OUTPUTS(0).value == 1000000L)",
        "height": 2000,
        "outputs": [ { "ergoTree": "100204000502d193c1b2a57300007301", "value": 1000000 } ]
    }"#;
    let out = eval_json(json);
    assert_eq!(out.verdict, Verdict::Pass, "error: {:?}", out.error);

    let json = r#"{
        "source": "sigmaProp(OUTPUTS(0).R4[Int].get == 7)",
        "height": 2000,
        "outputs": [ {
            "ergoTree": "100204000502d193c1b2a57300007301",
            "value": 100,
            "registers": { "R4": { "type": "Int", "value": 7 } }
        } ]
    }"#;
    let out = eval_json(json);
    assert_eq!(out.verdict, Verdict::Pass, "error: {:?}", out.error);
}

// ----- error paths -----

#[test]
fn runtime_exception_is_an_error_verdict_not_a_panic() {
    // OUTPUTS(0) with no outputs → box-index runtime exception.
    let out = eval_json(r#"{"source":"sigmaProp(OUTPUTS(0).value == 1L)","height":2000}"#);
    assert_eq!(out.verdict, Verdict::Error);
    let err = out.error.as_deref().expect("error text");
    assert!(err.contains("box index"), "error was {err}");
    // Cost still accounted.
    assert!(out.cost > 0);
}

#[test]
fn cost_limit_is_enforced_and_reported() {
    let out = eval_json(r#"{"source":"sigmaProp(HEIGHT > 100)","height":200,"costLimit":2}"#);
    assert_eq!(out.verdict, Verdict::Error);
    let err = out.error.as_deref().expect("error text");
    assert!(err.contains("cost limit"), "error was {err}");
    assert!(out.cost >= 2, "cost {} should meet the limit", out.cost);
    assert_eq!(out.cost_limit, 2);
}

#[test]
fn malformed_scenarios_fail_at_the_boundary() {
    // Missing both tree and source.
    let sc: Scenario = serde_json::from_str(r#"{"height":100}"#).expect("deserializes");
    assert!(eval_scenario(&sc).is_err());
    // Both tree and source.
    let sc: Scenario = serde_json::from_str(&format!(
        r#"{{"height":100,"tree":"{HEIGHT_TREE}","source":"sigmaProp(true)"}}"#
    ))
    .unwrap();
    assert!(eval_scenario(&sc).is_err());
    // Bad value/type pair.
    let sc: Scenario = serde_json::from_str(
        r#"{"height":100,"source":"sigmaProp(true)","contextVars":{"1":{"type":"Long","value":"nope"}}}"#,
    )
    .unwrap();
    assert!(eval_scenario(&sc).is_err());
    // Non-dense registers are rejected (valid tree, so the failure is the
    // density check — eval parses the tree before building boxes).
    let sc: Scenario = serde_json::from_str(&format!(
        r#"{{"height":100,"source":"sigmaProp(true)","outputs":[{{"ergoTree":"{HEIGHT_TREE}","registers":{{"R5":{{"type":"Int","value":1}}}}}}]}}"#
    ))
    .unwrap();
    assert!(matches!(
        eval_scenario(&sc),
        Err(ergo_sandbox::SandboxError::Scenario(_))
    ));
}

#[test]
fn unparseable_tree_bytes_are_a_boundary_error() {
    let sc: Scenario = serde_json::from_str(r#"{"height":100,"tree":"ffdead"}"#).unwrap();
    assert!(matches!(
        eval_scenario(&sc),
        Err(ergo_sandbox::SandboxError::Tree(_))
    ));
}

// ----- cost -----

#[test]
fn cost_is_nonzero_and_bounded_below_the_limit() {
    let out = eval_json(r#"{"source":"sigmaProp(HEIGHT > 100)","height":200}"#);
    assert!(out.cost > 0);
    assert!(out.cost <= out.cost_limit);
    assert_eq!(out.cost_limit, ergo_sandbox::eval::DEFAULT_COST_LIMIT);
}

#[test]
fn trivial_p2pk_tree_reports_the_signature_proposition() {
    // A bare ProveDlog tree — the mainnet P2PK shape (header 0x00, inline
    // SSigmaProp constant; taken from the diff corpus).
    const P2PK_TREE: &str =
        "0008cd034a53f17d249721c647c13477bb16982c8b2b16daa923d2a49dee8a88593c8356";
    let out = eval_json(&format!(r#"{{"tree":"{P2PK_TREE}","height":100}}"#));
    assert_eq!(out.verdict, Verdict::NeedsProof);
    let red = out.reduced_to.as_deref().unwrap_or_default();
    assert!(red.starts_with("ProveDlog(034a53f1"), "was {red}");
}

// ----- inspect -----

#[test]
fn tree_report_renders_and_roundtrips_the_seed_tree() {
    let bytes = hex::decode(HEIGHT_TREE).unwrap();
    let report = inspect::tree_report(&bytes).unwrap();
    assert!(report.contains("GT HEIGHT $0"), "report was:\n{report}");
    assert!(report.contains("byte-identical"));
    assert!(report.contains("$0: Int = 100"));
}

#[test]
fn decompile_recompile_byte_identity_over_the_compile_corpus() {
    // The P0 verification bar, exercised over a fixture sampled from the
    // ergo node's oracle-graded compile corpus
    // (arkadianet/ergo · test-vectors/ergoscript/compile/compile_seed.json,
    // Scala sigmastate 6.0.2 as the external oracle). Every entry is a real
    // Scala-accepted `(source, tree_bytes)` pair: parse the bytes, render the
    // structural view, and pin byte-exact re-serialization.
    const FIXTURE: &[(&str, &str)] = &[
        ("sigmaProp(HEIGHT > 100)", "100104c801d191a37300"),
        ("{ val x = HEIGHT; x > 5 }", "1001040ad191a37300"),
        ("true && (1 == 1)", "1000d1ed8503"),
        ("HEIGHT>5 && HEIGHT<9", "1002040a0412d1ed91a373008fa37301"),
        // (The corpus's `col1.exists(…)` vector needs the oracle's demo env
        //  (`col1` binding); skipped here — the sandbox compiles with an
        //  empty env. See arkadianet/ergo compile_semantic_parity.rs.)
        ("!true", "10010100d17300"),
        ("1 == 2", "100204020404d19373007301"),
        ("1 < 2L", "10010101d17300"),
        (
            "allOf(Coll(HEIGHT > 5, HEIGHT < 9))",
            "1002040a0412d19683020191a373008fa37301",
        ),
        ("anyOf(Coll(HEIGHT > 5))", "1001040ad191a37300"),
    ];
    for (source, hex_str) in FIXTURE {
        let bytes = hex::decode(hex_str).unwrap();
        // The identity claim needs BOTH halves: the source must compile to
        // exactly these bytes (catches source/tree mismatch or a compile
        // regression), and the bytes must re-serialize identically.
        let compiled =
            ergo_sandbox::compile_source(source, 3, ergo_ser::address::NetworkPrefix::Testnet)
                .unwrap_or_else(|e| panic!("fixture source {source} failed to compile: {e}"));
        assert_eq!(
            hex::encode(&compiled.tree_bytes),
            *hex_str,
            "fixture source {source} does not compile to the pinned tree"
        );
        let report = inspect::tree_report(&bytes)
            .unwrap_or_else(|e| panic!("fixture {source} failed to inspect: {e}"));
        assert!(
            report.contains("byte-identical"),
            "fixture {source} did not re-serialize byte-identical:\n{report}"
        );
    }
}

#[test]
fn opcode_name_covers_the_parser_supported_set_exactly() {
    use ergo_sandbox::inspect::opcode_name;
    // Exhaustive over the pinned ergo-ser `opcode_pattern` accept set
    // (the parse-accept set = the set that can appear in any valid tree):
    // every supported opcode byte has a name, and no unsupported byte does.
    const SUPPORTED: &[u8] = &[
        0x71, 0x72, 0x73, 0x74, 0x7A, 0x7B, 0x7C, 0x7D, 0x7E, 0x7F, 0x80, 0x82, 0x83, 0x85, 0x86,
        0x8C, 0x8F, 0x90, 0x91, 0x92, 0x93, 0x94, 0x95, 0x96, 0x97, 0x98, 0x99, 0x9A, 0x9B, 0x9C,
        0x9D, 0x9E, 0x9F, 0xA0, 0xA1, 0xA2, 0xA3, 0xA4, 0xA5, 0xA6, 0xA7, 0xAC, 0xAD, 0xAE, 0xAF,
        0xB0, 0xB1, 0xB2, 0xB3, 0xB4, 0xB5, 0xB6, 0xB7, 0xC1, 0xC2, 0xC3, 0xC4, 0xC5, 0xC6, 0xC7,
        0xCB, 0xCC, 0xCD, 0xCE, 0xCF, 0xD0, 0xD1, 0xD4, 0xD5, 0xD6, 0xD7, 0xD8, 0xD9, 0xDA, 0xDB,
        0xDC, 0xDD, 0xE3, 0xE4, 0xE5, 0xE6, 0xE7, 0xE8, 0xE9, 0xEA, 0xEB, 0xEC, 0xED, 0xEE, 0xEF,
        0xF0, 0xF1, 0xF2, 0xF3, 0xF4, 0xF5, 0xF6, 0xF7, 0xF8, 0xFE, 0xFF,
    ];
    for &op in SUPPORTED {
        assert!(
            opcode_name(op).is_some(),
            "0x{op:02X} is parser-supported but unnamed"
        );
    }
    // Reserved/unsupported bytes must stay unnamed (0x81 UnitConstant,
    // 0x87..=0x8B Select1..5, 0xB8 FlatMap, 0xDE SomeValue, 0xDF NoneValue —
    // none parser-accepted).
    for op in [0x81u8, 0x87, 0x88, 0x89, 0x8A, 0x8B, 0xB8, 0xDE, 0xDF] {
        assert!(
            opcode_name(op).is_none(),
            "0x{op:02X} unsupported but named"
        );
    }
}

// ----- raw (serialized) register values, as an explorer reports them -----

#[test]
fn a_raw_serialized_constant_parses_to_its_typed_value() {
    use ergo_sandbox::parse_typed_value;
    use ergo_ser::sigma_type::SigmaType;
    use ergo_ser::sigma_value::{CollValue, SigmaValue};
    // Int 5: type code 0x04, zigzag VLQ 10.
    let (t, v) = parse_typed_value("raw", &serde_json::json!("040a")).unwrap();
    assert_eq!(t, SigmaType::SInt);
    assert_eq!(v, SigmaValue::Int(5));
    // Coll[Byte] aa bb cc: type 0x0e, length 3.
    let (t, v) = parse_typed_value("raw", &serde_json::json!("0e03aabbcc")).unwrap();
    assert_eq!(t, SigmaType::SColl(Box::new(SigmaType::SByte)));
    assert_eq!(
        v,
        SigmaValue::Coll(CollValue::Bytes(vec![0xaa, 0xbb, 0xcc]))
    );
    assert!(parse_typed_value("raw", &serde_json::json!("zz")).is_err());
    assert!(
        parse_typed_value("raw", &serde_json::json!("040a00")).is_err(),
        "trailing bytes"
    );
}

#[test]
fn a_scenario_box_accepts_raw_registers() {
    let sc: ergo_sandbox::Scenario = serde_json::from_str(
        r#"{"source":"sigmaProp(SELF.R4[Int].get == 5)","height":1,
            "selfBox":{"registers":{"R4":{"type":"raw","value":"040a"}}}}"#,
    )
    .unwrap();
    assert_eq!(
        ergo_sandbox::eval_scenario(&sc).unwrap().verdict,
        ergo_sandbox::Verdict::Pass
    );
}
