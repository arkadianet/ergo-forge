use ergo_sandbox::{compile_source_raw, decompile};
use ergo_ser::address::NetworkPrefix;

fn round_trip(source: &str) -> String {
    let source = source.to_string();
    decompile::with_large_stack(move || {
        let original = compile_source_raw(&source, 3, NetworkPrefix::Testnet)
            .unwrap_or_else(|e| panic!("initial compile: {e}\n{source}"));
        let report = decompile::decompile_report(&original.tree_bytes, true).unwrap();
        assert_eq!(report.raw_placeholders, 0, "{}", report.source);
        let again = compile_source_raw(&report.source, 3, NetworkPrefix::Testnet)
            .unwrap_or_else(|e| panic!("recompile: {e}\n{}", report.source));
        assert_eq!(
            hex::encode(again.tree_bytes),
            hex::encode(original.tree_bytes),
            "{}",
            report.source
        );
        report.source
    })
}

#[test]
fn tuple_parameters_outside_fold_keep_their_arity() {
    for source in [
        "SELF.tokens.exists { (t: (Coll[Byte], Long)) => t._2 > 0L }",
        "SELF.tokens.map { (t: (Coll[Byte], Long)) => t }.size > 0",
        "{ val f = { (t: (Int, Int)) => t._1 + t._2 }; f((HEIGHT, 1)) > 0 }",
    ] {
        round_trip(source);
    }
}

#[test]
fn signature_operands_are_never_discarded_or_converted_to_boolean() {
    for source in [
        "proveDlog(SELF.R4[GroupElement].get)",
        "proveDlog(groupGenerator)",
        "proveDlog(if (HEIGHT > 0) groupGenerator else SELF.R4[GroupElement].get)",
        "{ val p = SELF.R4[SigmaProp].get; atLeast(2, Coll(p, p, p)) }",
        "sigmaProp(SELF.R4[SigmaProp].get.isProven)",
    ] {
        round_trip(source);
    }
}

#[test]
fn group_element_constants_keep_their_type() {
    round_trip("SELF.R4[GroupElement].get == decodePoint(fromBase16(\"0279be667ef9dcbbac55a06295ce870b07029bfcdb2dce28d959f2815b16f81798\"))");
}

#[test]
fn mixed_boolean_precedence_keeps_the_original_grouping() {
    for source in [
        "((HEIGHT > 0) ^ (HEIGHT > 1)) && HEIGHT > 2",
        "HEIGHT > 0 && ((HEIGHT > 1) ^ (HEIGHT > 2))",
        "((HEIGHT > 0) == (HEIGHT > 1)) ^ (HEIGHT > 2)",
        "Global.xor(SELF.id, SELF.propositionBytes) == SELF.bytes",
    ] {
        round_trip(source);
    }
}

#[test]
fn collection_index_and_default_keep_the_collection_receiver() {
    for source in [
        "CONTEXT.dataInputs(0).value > 0L",
        "SELF.R4[Coll[Int]].get.getOrElse(HEIGHT, 0) > 0",
        "OUTPUTS.getOrElse(HEIGHT, SELF).value > 0L",
    ] {
        round_trip(source);
    }
}

#[test]
fn register_zero_retains_its_option_type() {
    round_trip("SELF.R0[Long].get > 0L");
    round_trip("SELF.R0[Long].getOrElse(1L) > 0L");
}

#[test]
fn dh_tuple_and_constant_substitution_keep_all_operands() {
    round_trip("proveDHTuple(groupGenerator, SELF.R4[GroupElement].get, SELF.R5[GroupElement].get, SELF.R6[GroupElement].get)");
    round_trip("substConstants(SELF.propositionBytes, Coll(0), Coll(HEIGHT)).size > 0");
}

#[test]
fn compiler_retyping_requires_temporary_bindings() {
    for source in [
        "{ val x = SELF.R4[Int].getOrElse(0); HEIGHT >= x + 103 }",
        "{ val x = if (HEIGHT > 0 && HEIGHT < 10) 1L else 2L; SELF.value * x > 0L }",
        "{ val x = INPUTS.fold(0L, { (a: Long, b: Box) => a + b.value }); x * SELF.value > 0L }",
    ] {
        round_trip(source);
    }
}

#[test]
fn a_function_used_only_by_folds_keeps_byte_exact_shared_calls() {
    round_trip("{ val f = { (a: Long, b: Box) => a + b.value }; INPUTS.fold(0L, f) > OUTPUTS.fold(0L, f) }");
}

#[test]
fn miner_pubkey_uses_the_case_sensitive_source_name() {
    round_trip("proveDlog(decodePoint(MinerPubkey))");
}

use ergo_ser::{
    opcode::{Expr, IrNode, Payload},
    sigma_type::SigmaType,
    sigma_value::{CollValue, SigmaValue},
};

fn op(opcode: u8, payload: Payload) -> Expr {
    Expr::Op(IrNode { opcode, payload })
}
fn int(value: i32) -> Expr {
    Expr::Const {
        tpe: SigmaType::SInt,
        val: SigmaValue::Int(value),
    }
}
fn render_ir(body: Expr) -> decompile::Decompiled {
    let mut tree = compile_source_raw("true", 3, NetworkPrefix::Testnet)
        .unwrap()
        .ergo_tree;
    tree.body = body;
    tree.constants.clear();
    decompile::render_report_net(&tree, true)
}

#[test]
fn composite_constants_use_declared_types_including_empty_collections() {
    let report = render_ir(Expr::Const {
        tpe: SigmaType::STuple(vec![
            SigmaType::SInt,
            SigmaType::SColl(Box::new(SigmaType::SColl(Box::new(SigmaType::SLong)))),
        ]),
        val: SigmaValue::Tuple(vec![
            SigmaValue::Int(7),
            SigmaValue::Coll(CollValue::Values(vec![SigmaValue::Coll(
                CollValue::Values(vec![]),
            )])),
        ]),
    });
    assert_eq!(report.raw_placeholders, 0);
    assert_eq!(report.source, "(7, Coll(Coll[Long]()))");
}

#[test]
fn unavailable_information_is_always_a_visible_structural_placeholder() {
    let self_box = op(0xA7, Payload::Zero);
    for expr in [
        op(
            0xDB,
            Payload::MethodCall {
                type_id: 99,
                method_id: 7,
                obj: Box::new(self_box.clone()),
                args: vec![int(4)],
                type_args: vec![],
            },
        ),
        op(
            0xDB,
            Payload::MethodCall {
                type_id: 99,
                method_id: 250,
                obj: Box::new(self_box.clone()),
                args: vec![],
                type_args: vec![],
            },
        ),
        op(
            0xDB,
            Payload::MethodCall {
                type_id: 99,
                method_id: 1,
                obj: Box::new(self_box),
                args: vec![],
                type_args: vec![SigmaType::SLong],
            },
        ),
        op(
            0xD5,
            Payload::DeserializeRegister {
                reg_id: 4,
                tpe: SigmaType::SInt,
                default: Some(Box::new(int(1))),
            },
        ),
        op(
            0x72,
            Payload::TaggedVar {
                id: 1,
                tpe: Some(SigmaType::SInt),
            },
        ),
        op(
            0xE8,
            Payload::NoneValue {
                tpe: SigmaType::SInt,
            },
        ),
        op(0x72, Payload::ValUse { id: 42 }),
        op(0x73, Payload::ConstPlaceholder { index: 42 }),
        op(0xEA, Payload::SigmaCollection { items: vec![] }),
        op(
            0xEB,
            Payload::SigmaCollection {
                items: vec![op(0xD1, Payload::One(Box::new(int(1))))],
            },
        ),
        op(
            0xD7,
            Payload::FunDef {
                id: 1,
                tpe: None,
                tpe_args: vec![SigmaType::SInt],
                rhs: Box::new(int(1)),
            },
        ),
        Expr::Const {
            tpe: SigmaType::SUnit,
            val: SigmaValue::Unit,
        },
        Expr::Const {
            tpe: SigmaType::SSigmaProp,
            val: SigmaValue::SigmaProp(ergo_ser::sigma_value::SigmaBoolean::Cand(vec![])),
        },
    ] {
        let report = render_ir(expr);
        assert!(report.raw_placeholders > 0, "{}", report.source);
        assert!(
            report.source.starts_with('<') && report.source.ends_with('>'),
            "{}",
            report.source
        );
    }
}

#[test]
fn unsupported_block_items_are_not_silently_dropped() {
    let report = render_ir(op(
        0xD8,
        Payload::BlockValue {
            items: vec![int(7)],
            result: Box::new(int(1)),
        },
    ));
    assert_eq!(report.raw_placeholders, 1);
    assert!(
        report.source.contains("<unsupported block item:"),
        "{}",
        report.source
    );
}

#[test]
fn reused_binding_ids_keep_statement_order_and_rhs_scope() {
    let report = render_ir(op(
        0xD8,
        Payload::BlockValue {
            items: vec![
                op(
                    0xD6,
                    Payload::ValDef {
                        id: 1,
                        tpe: None,
                        rhs: Box::new(op(0xA3, Payload::Zero)),
                    },
                ),
                op(
                    0xD6,
                    Payload::ValDef {
                        id: 1,
                        tpe: None,
                        rhs: Box::new(op(
                            0x9A,
                            Payload::Two(
                                Box::new(op(0x72, Payload::ValUse { id: 1 })),
                                Box::new(int(1)),
                            ),
                        )),
                    },
                ),
            ],
            result: Box::new(op(0x72, Payload::ValUse { id: 1 })),
        },
    ));
    assert_eq!(report.raw_placeholders, 0);
    assert_eq!(report.source, "{ val v = HEIGHT; val v2 = v + 1; v2 }");
}

#[test]
fn references_do_not_leak_from_a_finished_lambda_scope() {
    let report = render_ir(op(
        0xD8,
        Payload::BlockValue {
            items: vec![op(
                0xD6,
                Payload::ValDef {
                    id: 1,
                    tpe: None,
                    rhs: Box::new(op(
                        0xD9,
                        Payload::FuncValue {
                            args: vec![(2, Some(SigmaType::SInt))],
                            body: Box::new(op(0x72, Payload::ValUse { id: 2 })),
                        },
                    )),
                },
            )],
            result: Box::new(op(0x72, Payload::ValUse { id: 2 })),
        },
    ));
    assert_eq!(report.raw_placeholders, 1);
    assert!(report.source.contains("<unbound val 2>"));
}

#[test]
fn signed_numeric_literals_preserve_the_value_before_casting() {
    for source in [
        "(-128).toByte == SELF.R4[Byte].get",
        "(-32768).toShort == SELF.R4[Short].get",
        "(-1L) == SELF.value",
    ] {
        round_trip(source);
    }
}

#[test]
fn a_tuple_function_with_mixed_uses_admits_the_fold_arity_gap() {
    let function = op(
        0xD9,
        Payload::FuncValue {
            args: vec![(
                2,
                Some(SigmaType::STuple(vec![SigmaType::SLong, SigmaType::SBox])),
            )],
            body: Box::new(op(
                0x8C,
                Payload::SelectField {
                    input: Box::new(op(0x72, Payload::ValUse { id: 2 })),
                    field_idx: 1,
                },
            )),
        },
    );
    let use_function = || op(0x72, Payload::ValUse { id: 1 });
    let zero = || Expr::Const {
        tpe: SigmaType::SLong,
        val: SigmaValue::Long(0),
    };
    let report = render_ir(op(
        0xD8,
        Payload::BlockValue {
            items: vec![op(
                0xD6,
                Payload::ValDef {
                    id: 1,
                    tpe: None,
                    rhs: Box::new(function),
                },
            )],
            result: Box::new(op(
                0x93,
                Payload::Two(
                    Box::new(op(
                        0xB0,
                        Payload::Three(
                            Box::new(op(0xA4, Payload::Zero)),
                            Box::new(zero()),
                            Box::new(use_function()),
                        ),
                    )),
                    Box::new(op(
                        0xDA,
                        Payload::FuncApply {
                            func: Box::new(use_function()),
                            args: vec![op(
                                0x86,
                                Payload::Tuple {
                                    items: vec![zero(), op(0xA7, Payload::Zero)],
                                },
                            )],
                        },
                    )),
                ),
            )),
        },
    ));
    assert_eq!(report.raw_placeholders, 1);
    assert!(
        report.source.contains("x: (Long, Box)"),
        "{}",
        report.source
    );
    assert!(report
        .source
        .contains("<fold function: shared source arity cannot be recovered exactly>"));
}

#[test]
fn source_temporaries_preserve_the_ast_and_do_not_capture_names() {
    use decompile::{Node, NodeKind};
    let source = "{ val x = SELF.R4[Int].getOrElse(0); HEIGHT >= x + 103 }";
    let compiled = compile_source_raw(source, 3, NetworkPrefix::Testnet).unwrap();
    let lifted = decompile::lift_tree(&compiled.ergo_tree, true);
    let NodeKind::Infix(">=", _, rhs) = &lifted.node.kind else {
        panic!("{:#?}", lifted.node)
    };
    assert!(
        matches!(rhs.kind, NodeKind::Infix("+", ..)),
        "source temporaries must stay in the printer"
    );

    let node = |kind| Node { id: 0, kind };
    let expr = node(NodeKind::Infix(
        "+",
        Box::new(node(NodeKind::Val("dcLeft0".into()))),
        Box::new(node(NodeKind::Method(
            Box::new(node(NodeKind::Val("option".into()))),
            "getOrElse".into(),
            vec![node(NodeKind::Int(0))],
        ))),
    ));
    let printed = decompile::print(&expr);
    assert!(printed.contains("val dcLeft0_ = dcLeft0"), "{printed}");
    round_trip(&format!(
        "{{ val dcLeft0 = HEIGHT; val option = SELF.R4[Int]; {printed} > 0 }}"
    ));
}
