//! Cross-contract identity discharge, including the exploited Dexy LP gap.
use ergo_sandbox::audit::{
    audit, audit_with_contracts, ContractSet, Execution, FindingStatus, InputContract, Severity,
};
use ergo_sandbox::compile::compile_with_params;
use ergo_sandbox::map::refs::{required_bindings, Binding};
use ergo_sandbox::{compile_source, lift_tree, Lifted, TypedValue};
use ergo_ser::address::NetworkPrefix;
use std::collections::BTreeMap;

fn lift(source: &str) -> Lifted {
    lift_tree(
        &compile_source(source, 3, NetworkPrefix::Testnet)
            .unwrap()
            .ergo_tree,
        true,
    )
}

fn corpus(path: &str) -> Lifted {
    let params: BTreeMap<String, BTreeMap<String, TypedValue>> =
        serde_json::from_str(include_str!("fixtures/map-audit/parameters.json")).unwrap();
    let source = std::fs::read_to_string(format!(
        "{}/../examples/contracts/{path}",
        env!("CARGO_MANIFEST_DIR")
    ))
    .unwrap();
    lift_tree(
        &compile_with_params(&source, &params[path], 3, NetworkPrefix::Testnet)
            .unwrap()
            .ergo_tree,
        true,
    )
}

fn input<'a>(name: &'a str, index: usize, lifted: &'a Lifted) -> InputContract<'a> {
    InputContract {
        name,
        execution: Execution::SpendingInput(index),
        lifted,
    }
}

fn singleton() -> BTreeMap<String, String> {
    [(
        "11".repeat(32),
        "Test premise: synthetic token emitted once in quantity 1".into(),
    )]
    .into()
}

fn assert_preserved(local: &Lifted, contextual: &ergo_sandbox::audit::ContextAudit) {
    let original = audit(local);
    assert_eq!(original.findings.len(), contextual.findings.len());
    for (a, b) in original.findings.iter().zip(&contextual.findings) {
        assert_eq!(format!("{a:?}"), format!("{:?}", b.finding));
    }
}

#[test]
fn phoenix_bank_binds_the_proxy_bank_successor() {
    let proxy_name = "hodlcoin/phoenix/phoenix_v1_hodltoken_proxy.es";
    let bank_name = "hodlcoin/phoenix/phoenix_v1_hodltoken_bank.es";
    let proxy = corpus(proxy_name);
    let bank = corpus(bank_name);
    let inputs = [input(bank_name, 0, &bank), input(proxy_name, 1, &proxy)];
    let tokens = BTreeMap::new();
    let set = ContractSet {
        inputs: &inputs,
        complete: true,
        singleton_tokens: &tokens,
    };
    let result = audit_with_contracts(&proxy, &set);
    assert!(
        result.context_usable,
        "set diagnostics: {:?}",
        inputs
            .iter()
            .map(|i| (
                i.name,
                i.execution,
                i.lifted.raw_placeholders,
                i.lifted.truncated
            ))
            .collect::<Vec<_>>()
    );
    assert_preserved(&proxy, &result);
    let f = result
        .findings
        .iter()
        .find(|f| {
            f.finding.lint == "unbound-box-reserves" && f.finding.snippet.starts_with("OUTPUTS(0).")
        })
        .unwrap();
    let FindingStatus::Discharged(e) = &f.status else {
        panic!(
            "bank successor not discharged; {:?}",
            required_bindings(&bank.node)
        );
    };
    assert_eq!(e.companion, bank_name);
    assert_eq!(e.site, "OUTPUTS(0)");
    assert_eq!(e.binding, Binding::SelfSuccessor);
    assert_eq!(e.identity, "SELF.propositionBytes");
    assert!(e.binding_ir_id.is_some());
    assert_eq!(
        result
            .findings
            .iter()
            .filter(|f| matches!(f.status, FindingStatus::Discharged(_)))
            .count(),
        1
    );
    println!(
        "Phoenix: {} retained findings, 1 discharged",
        result.findings.len()
    );
}

#[test]
fn dexy_lp_real_exploit_gap_stays_high_even_with_pool_companion() {
    let swap = corpus("dexy/lp/pool/swap.es");
    let pool = corpus("dexy/lp/pool/main.es");
    let decoy = lift("sigmaProp(true)");
    let tokens = BTreeMap::new();
    // Both the intended order and the exploit's decoy-before-real-pool order.
    for pool_index in [0, 2] {
        let mut inputs = vec![
            input("swap.es", 1, &swap),
            input("main.es", pool_index, &pool),
        ];
        if pool_index == 2 {
            inputs.push(input("decoy", 0, &decoy));
        }
        let result = audit_with_contracts(
            &swap,
            &ContractSet {
                inputs: &inputs,
                complete: true,
                singleton_tokens: &tokens,
            },
        );
        assert!(
            result.context_usable,
            "set diagnostics: {:?}",
            inputs
                .iter()
                .map(|i| (
                    i.name,
                    i.execution,
                    i.lifted.raw_placeholders,
                    i.lifted.truncated
                ))
                .collect::<Vec<_>>()
        );
        assert_preserved(&swap, &result);
        let f = result
            .findings
            .iter()
            .find(|f| {
                f.finding.lint == "unbound-box-reserves"
                    && f.finding.snippet.starts_with("INPUTS(0).")
            })
            .unwrap();
        assert_eq!(f.finding.severity, Severity::High);
        assert!(matches!(f.status, FindingStatus::Active));
    }
    println!("Dexy LP: INPUTS(0) remains active HIGH in both input orders");
}

#[test]
fn only_required_same_slot_bindings_discharge() {
    let reader = lift("sigmaProp(INPUTS(0).value > SELF.value)");
    let nft = format!("fromBase16(\"{}\")", "11".repeat(32));
    let pin = format!("INPUTS(0).tokens(0)._1 == {nft}");
    let tokens = singleton();
    for (predicate, expected) in [
        (pin.clone(), true),
        (format!("HEIGHT > 10 && {pin}"), true),
        (format!("if (HEIGHT > 10) {pin} else false"), true),
        (format!("if (HEIGHT > 10) {pin} else {pin}"), true),
        (format!("HEIGHT > 10 || {pin}"), false),
        (format!("if (HEIGHT > 10) {pin} else true"), false),
        (format!("!({pin})"), false),
        (format!("INPUTS(1).tokens(0)._1 == {nft}"), false),
        (format!("OUTPUTS(0).tokens(0)._1 == {nft}"), false),
        (
            format!("CONTEXT.dataInputs(0).tokens(0)._1 == {nft}"),
            false,
        ),
        (
            format!("INPUTS.exists {{ (b: Box) => b.tokens(0)._1 == {nft} }}"),
            false,
        ),
        (format!("{{ val unused = {pin}; HEIGHT > 10 }}"), false),
    ] {
        let binder = lift(&format!("sigmaProp({predicate})"));
        let inputs = [input("reader", 0, &reader), input("binder", 1, &binder)];
        let result = audit_with_contracts(
            &reader,
            &ContractSet {
                inputs: &inputs,
                complete: true,
                singleton_tokens: &tokens,
            },
        );
        assert_eq!(
            result
                .findings
                .iter()
                .any(|f| matches!(f.status, FindingStatus::Discharged(_))),
            expected,
            "{predicate}"
        );
    }
}

#[test]
fn incomplete_or_unproven_context_never_discharges() {
    let reader = lift("sigmaProp(INPUTS(0).value > SELF.value)");
    let binder = lift(&format!(
        "sigmaProp(INPUTS(0).tokens(0)._1 == fromBase16(\"{}\"))",
        "11".repeat(32)
    ));
    let mut partial = binder.clone();
    partial.raw_placeholders = 1;
    let tokens = singleton();
    let empty = BTreeMap::new();
    for (inputs, complete, supply) in [
        (
            vec![input("reader", 0, &reader), input("binder", 1, &binder)],
            false,
            &tokens,
        ),
        (
            vec![input("reader", 0, &reader), input("binder", 1, &partial)],
            true,
            &tokens,
        ),
        (
            vec![input("reader", 0, &reader), input("binder", 0, &binder)],
            true,
            &tokens,
        ),
        (
            vec![input("reader", 0, &reader), input("reader", 1, &binder)],
            true,
            &tokens,
        ),
        (vec![input("binder", 1, &binder)], true, &tokens),
        (vec![input("reader", 0, &reader)], true, &tokens),
        (
            vec![input("reader", 0, &reader), input("binder", 1, &binder)],
            true,
            &empty,
        ),
    ] {
        let result = audit_with_contracts(
            &reader,
            &ContractSet {
                inputs: &inputs,
                complete,
                singleton_tokens: supply,
            },
        );
        assert_preserved(&reader, &result);
        assert!(result
            .findings
            .iter()
            .all(|f| matches!(f.status, FindingStatus::Active)));
    }
}

#[test]
fn context_variable_indices_are_not_shared_between_inputs() {
    // Exercise the GetVar index shape accepted by the existing box-key reader.
    // Source `.get` wrappers are currently outside that local detector's scope.
    fn variable_index(n: &mut ergo_sandbox::Node) {
        match &mut n.kind {
            ergo_sandbox::NodeKind::ApplyFn(coll, args)
                if matches!(coll.kind, ergo_sandbox::NodeKind::Leaf("INPUTS"))
                    && args.len() == 1 =>
            {
                args[0].kind = ergo_sandbox::NodeKind::GetVar(0, "Int".into());
            }
            ergo_sandbox::NodeKind::Index(coll, idx, _)
                if matches!(coll.kind, ergo_sandbox::NodeKind::Leaf("INPUTS")) =>
            {
                idx.kind = ergo_sandbox::NodeKind::GetVar(0, "Int".into());
            }
            ergo_sandbox::NodeKind::Block(stmts, result) => {
                for stmt in stmts {
                    match stmt {
                        ergo_sandbox::decompile::Stmt::Val(_, n)
                        | ergo_sandbox::decompile::Stmt::Def(_, n) => variable_index(n),
                    }
                }
                variable_index(result);
            }
            ergo_sandbox::NodeKind::Infix(_, a, b) => {
                variable_index(a);
                variable_index(b);
            }
            ergo_sandbox::NodeKind::Prop(a, _) | ergo_sandbox::NodeKind::Method(a, _, _) => {
                variable_index(a)
            }
            _ => {}
        }
    }
    let mut reader = lift("sigmaProp(INPUTS(0).value > SELF.value)");
    let mut binder = lift(&format!(
        "sigmaProp(INPUTS(0).tokens(0)._1 == fromBase16(\"{}\"))",
        "11".repeat(32)
    ));
    variable_index(&mut reader.node);
    variable_index(&mut binder.node);
    let inputs = [input("reader", 0, &reader), input("binder", 1, &binder)];
    let tokens = singleton();
    let result = audit_with_contracts(
        &reader,
        &ContractSet {
            inputs: &inputs,
            complete: true,
            singleton_tokens: &tokens,
        },
    );
    assert!(result
        .findings
        .iter()
        .any(|f| f.finding.lint == "unbound-box-reserves"));
    assert!(result
        .findings
        .iter()
        .all(|f| matches!(f.status, FindingStatus::Active)));
}

#[test]
fn serialized_discharge_keeps_original_finding_and_names_its_proof() {
    let reader = lift("sigmaProp(INPUTS(0).value > SELF.value)");
    let binder = lift(&format!(
        "sigmaProp(INPUTS(0).tokens(0)._1 == fromBase16(\"{}\"))",
        "11".repeat(32)
    ));
    let tokens = singleton();
    let inputs = [input("reader", 0, &reader), input("binder", 1, &binder)];
    let set = ContractSet {
        inputs: &inputs,
        complete: true,
        singleton_tokens: &tokens,
    };
    let result = audit_with_contracts(&reader, &set);
    let json = serde_json::to_value(&result).unwrap();
    let finding = &json["findings"][0];
    assert_eq!(finding["lint"], "unbound-box-reserves");
    assert_eq!(finding["status"], "discharged");
    assert_eq!(finding["evidence"]["companion"], "binder");
    assert_eq!(finding["evidence"]["binding"], "nft");
    assert_eq!(finding["evidence"]["site"], "INPUTS(0)");
    assert_eq!(finding["evidence"]["identity"], "11".repeat(32));
    assert!(!finding["evidence"]["binding_ir_id"].is_null());
    // Input ordering cannot choose a different discharge or change the output.
    let reversed = [inputs[1], inputs[0]];
    let reversed = audit_with_contracts(
        &reader,
        &ContractSet {
            inputs: &reversed,
            ..set
        },
    );
    assert_eq!(json, serde_json::to_value(reversed).unwrap());
}

#[test]
fn script_identity_cannot_discharge_data_provider_provenance() {
    let reader = lift("sigmaProp(CONTEXT.dataInputs(0).R4[Long].get > 0L)");
    let binder = lift(&format!(
        "sigmaProp(blake2b256(CONTEXT.dataInputs(0).propositionBytes) == fromBase16(\"{}\"))",
        "11".repeat(32)
    ));
    let inputs = [input("reader", 0, &reader), input("binder", 1, &binder)];
    let tokens = singleton();
    let result = audit_with_contracts(
        &reader,
        &ContractSet {
            inputs: &inputs,
            complete: true,
            singleton_tokens: &tokens,
        },
    );
    assert!(result
        .findings
        .iter()
        .any(|f| f.finding.lint == "trust-assumptions"));
    assert!(result
        .findings
        .iter()
        .all(|f| matches!(f.status, FindingStatus::Active)));
}
