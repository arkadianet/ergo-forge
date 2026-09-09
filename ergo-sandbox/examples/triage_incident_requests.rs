use serde_json::{json, Value};
fn main() {
    let fx: Value = serde_json::from_str(include_str!(
        "../../examples/incidents/use-lp-drain.deployed-swap.test.json"
    ))
    .unwrap();
    let sc = &fx["scenarios"][0];
    let mut pool = sc["inputs"][2].clone();
    let mut swap = sc["inputs"][1].clone();
    let mut successor = pool.clone();
    successor["creationHeight"] = json!(1868202);
    successor["payee"] = json!("fixed");
    pool["role"] = json!("protected");
    swap["role"] = json!("companion");
    let mut swap_out = sc["outputs"][1].clone();
    swap_out["payee"] = json!("fixed");
    let mut fee = sc["outputs"][3].clone();
    fee["payee"] = json!("fixed");
    let trader = &sc["inputs"][0]["ergoTree"];
    let mut drain = json!({
        "inputs": [pool, swap, {"role":"attacker","value":2000000,"ergoTree":trader}],
        "outputs": [successor, swap_out, {"payee":"free","value":0,"ergoTree":trader,"tokens":[]},fee],
        "protocolNfts": [sc["inputs"][2]["tokens"][0]["id"]],
        "height":1868204,"network":"mainnet","objective":{"terms":[]},
        "maxProbes":50000,"maxPermutations":120
    });
    let bytes = hex::decode(drain["inputs"][0]["ergoTree"].as_str().unwrap()).unwrap();
    let tree = ergo_sandbox::inspect::parse_tree(&bytes).unwrap();
    let audit = ergo_sandbox::audit::audit(&ergo_sandbox::lift_tree(&tree, false));
    for f in &audit.findings {
        println!("{} {}", f.lint, f.node_id);
    }
    let finding = audit
        .findings
        .iter()
        .find(|f| f.lint == "delegated-reserves")
        .unwrap();
    let mut req =
        json!({"inputIndex":0,"lint":finding.lint,"nodeId":finding.node_id,"drain":drain});
    std::fs::write(
        "examples/incidents/use-lp.triage.json",
        serde_json::to_string_pretty(&req).unwrap() + "\n",
    )
    .unwrap();
    let fixed: Value = serde_json::from_str(include_str!(
        "../../examples/incidents/use-lp-drain.fixed-swap.test.json"
    ))
    .unwrap();
    let compiled = ergo_sandbox::compile::compile_with_params(
        include_str!("../../examples/incidents/fixed/use-lp-swap.es"),
        &serde_json::from_value(fixed["params"].clone()).unwrap(),
        3,
        ergo_ser::address::NetworkPrefix::Mainnet,
    )
    .unwrap();
    drain["inputs"][1]["ergoTree"] = json!(hex::encode(compiled.tree_bytes));
    req["drain"] = drain;
    std::fs::write(
        "examples/incidents/fixed/use-lp.triage.json",
        serde_json::to_string_pretty(&req).unwrap() + "\n",
    )
    .unwrap();
}
