use ergo_sandbox::{audit, compile_source, lift_tree, Finding, Severity};
use ergo_ser::address::NetworkPrefix;

const PK: &str = "PK(\"9hHondX3uZMY2wQsXuCGjbgZUqunQyZCNNuwGu6rL7AJC8dhRGa\").propBytes";

fn check(condition: &str, severity: Severity) {
    let compiled = compile_source(
        &format!("{{ val out = OUTPUTS(0); sigmaProp({condition}) }}"),
        3,
        NetworkPrefix::Mainnet,
    )
    .expect("compile");
    for inline in [false, true] {
        let lifted = lift_tree(&compiled.ergo_tree, inline);
        let f: Vec<Finding> = audit::lints::unbound_box_reserves(&lifted.node)
            .into_iter()
            .filter(|f| f.snippet.starts_with("OUTPUTS(0)"))
            .collect();
        assert_eq!(f.len(), 1, "{condition}: {f:?}");
        assert_eq!(f[0].severity, severity, "{condition}: {f:?}");
        if severity == Severity::Low {
            assert!(f[0].message.contains("payment-output constraints"));
            assert!(f[0]
                .message
                .contains("ordinary payment does not require one"));
        } else {
            assert_eq!(f[0].message, "reserves of OUTPUTS(0) drive value math but this box is never bound by a singleton NFT (no tokens(0)._1 == <nft> check); a transaction can place a different box at this index. Bind it by its NFT.");
        }
    }
}

#[test]
fn fixed_script_and_required_value_reclassify_but_retain_payment_findings() {
    for script in [PK, "fromBase16(\"aabb\")"] {
        for value in [
            "out.value >= 1000000L",
            "out.value == SELF.value / 3L",
            "1000000L <= out.value",
            "SELF.value / 3L == out.value",
            "out.value > 1000000L",
        ] {
            for identity in [
                format!("out.propositionBytes == {script}"),
                format!("{script} == out.propositionBytes"),
            ] {
                check(&format!("{identity} && {value}"), Severity::Low);
            }
        }
    }
}

#[test]
fn incomplete_payment_evidence_and_pool_math_keep_original_high_finding() {
    for condition in [
        "out.value * out.tokens(1)._2 >= INPUTS(0).value * INPUTS(0).tokens(1)._2".to_owned(),
        "out.value >= 1000000L".to_owned(),
        format!("out.propositionBytes == {PK} && out.value <= 1000000L"),
        format!("out.propositionBytes == {PK} && out.value != 1000000L"),
        format!("out.propositionBytes == {PK} && out.value == out.value"),
        format!("out.propositionBytes == {PK} || out.value >= 1000000L"),
        "out.propositionBytes == INPUTS(0).propositionBytes && out.value >= 1000000L".to_owned(),
        "out.propositionBytes == getVar[Coll[Byte]](0).get && out.value >= 1000000L".to_owned(),
        format!(
            "out.propositionBytes == {PK} && OUTPUTS(1).value >= 1000000L && out.value * 2L > 0L"
        ),
    ] {
        check(&condition, Severity::High);
    }
}

#[test]
fn confirmed_corpus_payments_are_retained_as_low() {
    let cases = [
        (
            include_str!("../../examples/contracts/recipes/nft-sale.es"),
            include_str!("../../examples/tests/nft-sale.test.json"),
            vec![0, 1],
        ),
        (
            include_str!("../../examples/contracts/recipes/token-sale.es"),
            include_str!("../../examples/tests/token-sale.test.json"),
            vec![0],
        ),
        (
            include_str!("../../examples/contracts/recipes/subscription.es"),
            include_str!("../../examples/tests/subscription.test.json"),
            vec![0],
        ),
        (
            include_str!("../../examples/contracts/protocols/registry/registry.es"),
            include_str!("../../examples/tests/registry.test.json"),
            vec![1],
        ),
        (
            include_str!("../../examples/contracts/dexy/hodlcoin/hodlcoin.es"),
            "{\"params\":{}}",
            vec![1, 2, 3],
        ),
    ];
    for (source, fixture, outputs) in cases {
        let fixture: serde_json::Value = serde_json::from_str(fixture).unwrap();
        let params = serde_json::from_value(fixture["params"].clone()).unwrap();
        let compiled =
            ergo_sandbox::compile::compile_with_params(source, &params, 3, NetworkPrefix::Mainnet)
                .expect("compile corpus payment");
        for inline in [false, true] {
            let lifted = lift_tree(&compiled.ergo_tree, inline);
            let findings = audit::lints::unbound_box_reserves(&lifted.node);
            for index in &outputs {
                let matching: Vec<_> = findings
                    .iter()
                    .filter(|f| f.snippet == format!("OUTPUTS({index}).value"))
                    .collect();
                assert_eq!(matching.len(), 1, "OUTPUTS({index}): {findings:?}");
                assert_eq!(matching[0].severity, Severity::Low, "{findings:?}");
            }
        }
    }
}
