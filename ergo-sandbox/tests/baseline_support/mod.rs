//! Preserve byte-pinned hunt baselines when S02 adds its separate namespace.

use std::path::Path;

use serde_json::Value;

pub fn historical_bytes(workspace: &Path, path: &str) -> Vec<u8> {
    let current = std::fs::read(workspace.join(path)).unwrap();
    if path != "examples/mutants/answer-key.json" {
        return current;
    }
    // Both M00 and D00 keep their original manifest and answer-key digest.
    // Pin the archived bytes AND require every live historical field to match;
    // only the separately gated S02 appendix is outside the hunt measurement.
    let historical =
        std::fs::read(workspace.join("examples/mutants/answer-key.pre-s02.json")).unwrap();
    let mut live: Value = serde_json::from_slice(&current).unwrap();
    assert!(live
        .as_object_mut()
        .unwrap()
        .remove("staticLintPairs")
        .is_some());
    assert_eq!(
        live,
        serde_json::from_slice::<Value>(&historical).unwrap(),
        "every historical answer-key field must remain unchanged"
    );
    historical
}
