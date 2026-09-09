use ergo_sandbox::{
    compile::compile_with_params,
    identity::{match_trees, MatchVerdict},
    TypedValue,
};
use ergo_ser::address::NetworkPrefix;
use serde_json::json;

fn compile(src: &str) -> Vec<u8> {
    ergo_sandbox::compile_source(src, 3, NetworkPrefix::Mainnet)
        .unwrap()
        .tree_bytes
}

#[test]
fn exact_substitution_and_operator_change() {
    let a = compile("sigmaProp(HEIGHT > 100)");
    let b = compile("sigmaProp(HEIGHT > 200)");
    let c = compile("sigmaProp(HEIGHT < 100)");
    assert_eq!(
        match_trees(&a, &a).unwrap().verdict,
        MatchVerdict::SameProgram
    );
    let r = match_trees(&a, &b).unwrap();
    assert_eq!(r.verdict, MatchVerdict::SameProgramWithDifferingConstants);
    assert_eq!(r.similarity, 1.0);
    assert_eq!(r.constant_differences.len(), 1);
    let d = &r.constant_differences[0];
    assert_eq!(d.left.as_ref().unwrap().r#type, "Int");
    assert!(d.left.as_ref().unwrap().value.contains("100"));
    assert!(d.right.as_ref().unwrap().value.contains("200"));
    assert_eq!(
        match_trees(&a, &c).unwrap().verdict,
        MatchVerdict::DifferentProgram
    );
    assert!(match_trees(&a, &c).unwrap().similarity < 1.0);
}

#[test]
fn selectors_types_bindings_and_order_are_structure() {
    for (a, b) in [
        (
            "sigmaProp(SELF.R4[Int].get > 0)",
            "sigmaProp(SELF.R5[Int].get > 0)",
        ),
        (
            "sigmaProp(SELF.R4[Int].get > 0)",
            "sigmaProp(SELF.R4[Long].get > 0L)",
        ),
        (
            "sigmaProp(getVar[Int](0).get > 0)",
            "sigmaProp(getVar[Int](1).get > 0)",
        ),
        (
            "sigmaProp(HEIGHT > SELF.value)",
            "sigmaProp(SELF.value > HEIGHT)",
        ),
        (
            "sigmaProp(HEIGHT > 100)",
            "sigmaProp(HEIGHT > 100 && SELF.value > 0)",
        ),
    ] {
        assert_eq!(
            match_trees(&compile(a), &compile(b)).unwrap().verdict,
            MatchVerdict::DifferentProgram,
            "{a} / {b}"
        );
    }
}

#[test]
fn malformed_and_unsupported_trees_never_match() {
    let a = compile("sigmaProp(HEIGHT > 100)");
    for bad in [
        vec![],
        vec![0],
        vec![0x09, 0x01, 0xff],
        hex::decode("1000d17300").unwrap(), // unresolved table index
        [a.as_slice(), &[0]].concat(),
        a[..a.len() - 1].to_vec(),
    ] {
        assert!(match_trees(&bad, &bad).is_err(), "{bad:?}");
        assert!(match_trees(&a, &bad).is_err());
    }
}

#[test]
fn dexy_deployed_swap() {
    let source = include_str!("../../examples/contracts/dexy/lp/pool/swap.es");
    let params = serde_json::from_value::<std::collections::BTreeMap<String, TypedValue>>(json!({
        "feeNumLp": {"type":"Long","value":3},
        "feeDenomLp": {"type":"Int","value":1000}
    }))
    .unwrap();
    let a = compile_with_params(source, &params, 3, NetworkPrefix::Mainnet).unwrap();
    let fixture: serde_json::Value = serde_json::from_str(include_str!(
        "../../examples/incidents/use-lp-drain.deployed-swap.test.json"
    ))
    .unwrap();
    let b = hex::decode(fixture["tree"].as_str().unwrap()).unwrap();
    let r = match_trees(&a.tree_bytes, &b).unwrap();
    println!("Dexy: {}", serde_json::to_string(&r).unwrap());
    assert_eq!(r.verdict, MatchVerdict::SameProgramWithDifferingConstants);
    assert!(r
        .constant_differences
        .iter()
        .any(|d| d.left.as_ref().unwrap().r#type != d.right.as_ref().unwrap().r#type));
    // Int numerator adds two cast nodes. Do not erase them to force a match.
    let mut int_params = params.clone();
    int_params.get_mut("feeNumLp").unwrap().r#type = "Int".into();
    let int_source = compile_with_params(source, &int_params, 3, NetworkPrefix::Mainnet).unwrap();
    let int_report = match_trees(&int_source.tree_bytes, &b).unwrap();
    assert_eq!(int_report.verdict, MatchVerdict::DifferentProgram);
    assert!(int_report.similarity < 1.0);
    let negative = match_trees(&compile("sigmaProp(HEIGHT > 100)"), &b).unwrap();
    assert_eq!(negative.verdict, MatchVerdict::DifferentProgram);
}

#[test]
fn inline_constants_resolve_by_occurrence() {
    use ergo_primitives::writer::VlqWriter;
    use ergo_ser::{
        ergo_tree::write_ergo_tree,
        opcode::{Expr, Payload},
    };
    let a = compile("sigmaProp(HEIGHT > 100 && SELF.value > 200L)");
    let mut tree = ergo_sandbox::inspect::parse_tree(&a).unwrap();
    fn inline(
        e: &mut Expr,
        cs: &[(
            ergo_ser::sigma_type::SigmaType,
            ergo_ser::sigma_value::SigmaValue,
        )],
    ) {
        // Fixture has only fixed-arity operations; fail if its shape changes.
        if let Expr::Op(n) = e {
            match &mut n.payload {
                Payload::ConstPlaceholder { index } => {
                    let (tpe, val) = cs[*index as usize].clone();
                    *e = Expr::Const { tpe, val };
                }
                Payload::One(a) => inline(a, cs),
                Payload::Two(a, b) => {
                    inline(a, cs);
                    inline(b, cs);
                }
                Payload::NumericCast { input, .. } => inline(input, cs),
                Payload::Zero => (),
                p => panic!("unexpected {p:?}"),
            }
        }
    }
    inline(&mut tree.body, &tree.constants);
    tree.constant_segregation = false;
    tree.constants.clear();
    let mut w = VlqWriter::new();
    write_ergo_tree(&mut w, &tree).unwrap();
    let r = match_trees(&a, &w.result()).unwrap();
    assert_eq!(r.verdict, MatchVerdict::SameProgram);
    assert!(!r.byte_identical);
}

#[test]
fn phoenix_deployments_and_near_miss_siblings() {
    ergo_sandbox::decompile::with_large_stack(|| {
        let fixture: serde_json::Value =
            serde_json::from_str(include_str!("fixtures/identity_phoenix.json")).unwrap();
        let params =
            serde_json::from_value::<std::collections::BTreeMap<String, TypedValue>>(json!({
                "phoenixFeeContractBytes": {"type":"Coll[Byte]","value":"00010203"},
                "phoenixFeeContractBytesHash": {"type":"Coll[Byte]","value":"00".repeat(32)},
                "minTxOperatorFee": {"type":"Long","value":2000000}
            }))
            .unwrap();
        for (name, source, sibling) in [
            (
                "bank",
                include_str!(
                    "../../examples/contracts/hodlcoin/phoenix/phoenix_v1_hodltoken_bank.es"
                ),
                include_str!(
                    "../../examples/contracts/hodlcoin/phoenix/phoenix_v1_hodlcoin_bank.es"
                ),
            ),
            (
                "proxy",
                include_str!(
                    "../../examples/contracts/hodlcoin/phoenix/phoenix_v1_hodltoken_proxy.es"
                ),
                include_str!(
                    "../../examples/contracts/hodlcoin/phoenix/phoenix_v1_hodlcoin_proxy.es"
                ),
            ),
        ] {
            let deployed = hex::decode(fixture[name]["tree"].as_str().unwrap()).unwrap();
            let compiled = compile_with_params(source, &params, 3, NetworkPrefix::Mainnet).unwrap();
            let r = match_trees(&compiled.tree_bytes, &deployed).unwrap();
            println!("Phoenix {name}: {}", serde_json::to_string(&r).unwrap());
            assert_eq!(
                r.verdict,
                MatchVerdict::SameProgramWithDifferingConstants,
                "{name}"
            );
            let sibling = compile_with_params(sibling, &params, 3, NetworkPrefix::Mainnet).unwrap();
            let r = match_trees(&sibling.tree_bytes, &deployed).unwrap();
            println!(
                "Phoenix {name} sibling: {}",
                serde_json::to_string(&r).unwrap()
            );
            assert_eq!(r.verdict, MatchVerdict::DifferentProgram, "{name} sibling");
        }
    });
}

#[test]
fn table_indices_are_storage_but_casts_and_versions_are_structure() {
    use ergo_primitives::writer::VlqWriter;
    use ergo_ser::{
        ergo_tree::write_ergo_tree,
        opcode::{Expr, Payload},
        sigma_type::SigmaType,
        sigma_value::SigmaValue,
    };
    let a = compile("sigmaProp(HEIGHT > 100)");
    let mut tree = ergo_sandbox::inspect::parse_tree(&a).unwrap();
    let original = tree.constants[0].clone();
    tree.constants[0] = (SigmaType::SInt, SigmaValue::Int(999));
    tree.constants.push(original);
    let Expr::Op(root) = &mut tree.body else {
        panic!()
    };
    let Payload::One(comparison) = &mut root.payload else {
        panic!()
    };
    let Expr::Op(comparison) = &mut **comparison else {
        panic!()
    };
    let Payload::Two(_, leaf) = &mut comparison.payload else {
        panic!()
    };
    let Expr::Op(leaf) = &mut **leaf else {
        panic!()
    };
    leaf.payload = Payload::ConstPlaceholder { index: 1 };
    let mut w = VlqWriter::new();
    write_ergo_tree(&mut w, &tree).unwrap();
    assert_eq!(
        match_trees(&a, &w.result()).unwrap().verdict,
        MatchVerdict::SameProgram
    );
    tree.version = 1;
    tree.has_size = true;
    let mut w = VlqWriter::new();
    write_ergo_tree(&mut w, &tree).unwrap();
    assert_eq!(
        match_trees(&a, &w.result()).unwrap().verdict,
        MatchVerdict::DifferentProgram
    );
    assert_eq!(
        match_trees(&a, &compile("sigmaProp(HEIGHT.toLong > 100L)"))
            .unwrap()
            .verdict,
        MatchVerdict::DifferentProgram
    );
}

#[test]
fn public_keys_and_boolean_literals_are_typed_holes() {
    // Root SigmaProp constants, including complete public key values.
    let a = hex::decode("0008cd0279be667ef9dcbbac55a06295ce870b07029bfcdb2dce28d959f2815b16f81798")
        .unwrap();
    let b = hex::decode("0008cd0379be667ef9dcbbac55a06295ce870b07029bfcdb2dce28d959f2815b16f81798")
        .unwrap();
    let r = match_trees(&a, &b).unwrap();
    assert_eq!(r.verdict, MatchVerdict::SameProgramWithDifferingConstants);
    assert_eq!(
        r.constant_differences[0].left.as_ref().unwrap().r#type,
        "SigmaProp"
    );
    let r = match_trees(&compile("sigmaProp(true)"), &compile("sigmaProp(false)")).unwrap();
    assert_eq!(r.verdict, MatchVerdict::SameProgramWithDifferingConstants);
}
