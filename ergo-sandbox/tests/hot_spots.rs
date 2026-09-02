//! Cost hot-spots: the per-step cost trace folded into a ranked view.

use ergo_sandbox::eval::CostLine;
use ergo_sandbox::hot_spots::{describe_label, hot_spots};

fn line(label: &str, delta: u64) -> CostLine {
    CostLine {
        label: label.to_string(),
        delta,
        total: 0,
    }
}

#[test]
fn opcode_labels_are_named_and_keep_their_detail() {
    assert_eq!(describe_label("OP:0xED"), "BinAnd (0xED)");
    assert_eq!(describe_label("OP:0xAE:n=1"), "Exists (0xAE) n=1");
    assert_eq!(describe_label("AddToEnv"), "AddToEnv");
    assert_eq!(describe_label("OP:0xZZ"), "OP:0xZZ");
}

#[test]
fn hot_spots_group_by_label_and_rank_by_cost() {
    let lines = [
        line("OP:0xED", 20),
        line("OP:0xC1", 8),
        line("OP:0xED", 20),
        line("EQ", 1),
        line("OP:0xC1", 8),
        line("OP:0xED", 20),
    ];
    let hs = hot_spots(&lines);
    let names: Vec<&str> = hs.iter().map(|h| h.label.as_str()).collect();
    assert_eq!(names, ["BinAnd (0xED)", "ExtractAmount (0xC1)", "EQ"]);
    assert_eq!(hs[0].jit, 60);
    assert_eq!(hs[0].count, 3);
    assert_eq!(hs[1].jit, 16);
    assert!((hs[0].share - 60.0 / 77.0).abs() < 1e-9);
}

#[test]
fn opcode_variants_with_detail_are_separate_rows() {
    let lines = [
        line("OP:0xAE:n=1", 4),
        line("OP:0xAE:n=8", 12),
        line("OP:0xAE:n=1", 4),
    ];
    let hs = hot_spots(&lines);
    assert_eq!(hs.len(), 2);
    assert_eq!(hs[0].label, "Exists (0xAE) n=8");
    assert_eq!(hs[1].count, 2);
}

#[test]
fn an_empty_trace_has_no_hot_spots() {
    assert!(hot_spots(&[]).is_empty());
}

/// With the feature on, the ranked view must account for every unit the
/// real evaluator charged.
#[cfg(feature = "cost-trace")]
#[test]
fn hot_spots_of_a_real_evaluation_sum_to_the_trace_total() {
    let sc: ergo_sandbox::Scenario = serde_json::from_str(
        r#"{"source":"sigmaProp(OUTPUTS.exists { (o: Box) => o.value >= SELF.value } && HEIGHT > 100)",
            "height":200,"selfBox":{"value":1000},"outputs":[{"value":2000,"ergoTree":"10010101"}]}"#,
    )
    .unwrap();
    let out = ergo_sandbox::eval_scenario(&sc).unwrap();
    assert_eq!(out.verdict, ergo_sandbox::Verdict::Pass);
    let hs = hot_spots(&out.cost_breakdown);
    let jit_total: u64 = out.cost_breakdown.iter().map(|c| c.delta).sum();
    assert!(jit_total > 0);
    assert_eq!(hs.iter().map(|h| h.jit).sum::<u64>(), jit_total);
    let share_total: f64 = hs.iter().map(|h| h.share).sum();
    assert!((share_total - 1.0).abs() < 1e-9, "{share_total}");
}
