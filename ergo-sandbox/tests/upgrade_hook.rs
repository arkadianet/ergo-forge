use ergo_sandbox::{audit::lints::upgrade_hook, compile_source, lift_tree};
use ergo_ser::address::NetworkPrefix;
fn findings(source: &str) -> Vec<ergo_sandbox::Finding> {
    let c = compile_source(source, 3, NetworkPrefix::Testnet).unwrap();
    upgrade_hook(&lift_tree(&c.ergo_tree, false).node)
}
const HOOK: &str = "blake2b256(OUTPUTS(0).propositionBytes) == SELF.R4[Coll[Byte]].get";
const SAME: &str = "OUTPUTS(0).propositionBytes == SELF.propositionBytes";
const CARRY: &str = "OUTPUTS(0).R4[Coll[Byte]].get == SELF.R4[Coll[Byte]].get";
#[test]
fn branch_local_carry_and_sigma_authority() {
    let pk = "PK(\"3WwbzW6u8hKWBcL1W7kNVMr25s2UHfSBnYtwSHvrRQt7DdPuoXrt\")";
    let f = findings(&format!("sigmaProp({HOOK}) || (sigmaProp({SAME}) && {pk})"));
    assert_eq!(f.len(), 1);
    assert!(f[0].message.contains("sigma guard"), "{f:?}");
    assert!(!f[0].message.contains("unguarded"), "{f:?}");
    assert!(f[0].message.contains("PK(") || f[0].message.contains("proveDlog"));
    assert!(findings(&format!("sigmaProp({HOOK} || ({SAME} && {CARRY}))")).is_empty());
    assert_eq!(
        findings(&format!("sigmaProp(({HOOK} && {CARRY}) || {SAME})")).len(),
        1
    );
    assert_eq!(
        findings(&format!(
            "sigmaProp({HOOK} || ({SAME} && {}))",
            CARRY.replace("OUTPUTS(0)", "OUTPUTS(1)")
        ))
        .len(),
        1
    );
    let guarded = format!("sigmaProp({HOOK}) || (sigmaProp({SAME}) && SELF.R5[SigmaProp].get)");
    assert!(findings(&guarded)[0]
        .message
        .contains("SELF.R5[SigmaProp].get"));
}
#[test]
fn positional_bytes_hooks_and_unrelated_register_controls() {
    let s = "sigmaProp(OUTPUTS(1).propositionBytes == INPUTS(1).R4[Coll[Byte]].get || OUTPUTS(1).propositionBytes == INPUTS(1).propositionBytes)";
    let f = findings(s);
    assert_eq!(f.len(), 1);
    assert!(f[0].message.contains("INPUTS(1).R4"));
    assert_eq!(
        findings(&s.replace("INPUTS(1)", "CONTEXT.dataInputs(0)")).len(),
        1
    );
    assert!(findings(&format!("sigmaProp({HOOK})")).is_empty());
    assert!(findings(&format!("{{ val unused = {HOOK}; sigmaProp({SAME}) }}")).is_empty());
    assert!(
        findings("sigmaProp(OUTPUTS(0).R4[Coll[Byte]].get == SELF.R4[Coll[Byte]].get)").is_empty()
    );
}
#[test]
fn checklist_upgrade_hook_has_static_provenance() {
    let c = compile_source(
        &format!("sigmaProp({HOOK} || {SAME})"),
        3,
        NetworkPrefix::Mainnet,
    )
    .unwrap();
    let result = ergo_sandbox::checklist::checklist(
        &c.tree_bytes,
        NetworkPrefix::Mainnet,
        &Default::default(),
    )
    .unwrap();
    let row = result
        .rows
        .iter()
        .find(|r| r.id == "mutable-upgrade-digest")
        .unwrap();
    assert_eq!(row.provenance, ergo_sandbox::checklist::Provenance::Static);
    assert!(row
        .findings
        .iter()
        .any(|f| f.lint == "upgrade-hook" && f.anchor.ir_id.is_some()));
}

#[test]
fn token_transfer_to_hook_destination_does_not_imply_same_script_continuation() {
    let s = format!("sigmaProp({HOOK} && OUTPUTS(0).tokens(0)._1 == SELF.tokens(0)._1)");
    assert!(findings(&s).is_empty());
}
