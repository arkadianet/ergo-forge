//! One-time raw D00 observation capture after manifest and answers were locked.
#[path = "../tests/property_support/mod.rs"]
mod property_support;
fn main() {
    let (m, _) = property_support::inventory();
    let output = property_support::measure(&m);
    let path = property_support::root().join("legacy-results.json");
    assert!(
        !path.exists(),
        "measurement is immutable; use a new version"
    );
    let raw_path = property_support::root().join("legacy-results-raw.fixture");
    assert!(
        !raw_path.exists(),
        "raw measurement is immutable; use a new version"
    );
    std::fs::write(raw_path, serde_json::to_vec_pretty(&output).unwrap()).unwrap();
    std::fs::write(
        path,
        serde_json::to_vec_pretty(&property_support::legacy_summary(&output)).unwrap(),
    )
    .unwrap();
    println!("{} individually accepted reference transactions, {} node API calls including fresh legacy replay; no linked-trace replay or new-property verdict", output["referenceTransactions"], output["nodeValidationCalls"]);
}
