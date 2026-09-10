//! Pure offline replay. No source resolution, network request, signing or broadcast.
use super::{
    claim::{self, Property, PropertyResult},
    validate::{validate, ValidationRequest},
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ReplayBundle {
    pub format_version: u32,
    pub execution: ValidationRequest,
    pub property: Property,
}
impl ReplayBundle {
    pub fn fingerprint(&self) -> String {
        super::case::json_digest(&serde_json::to_value(self).expect("bundle serializes"))
    }
}
/// Stored reports are not ReplayBundles. Replay their `bundle` after parsing;
/// neither an accepted flag nor a previous property verdict is imported.
pub fn replay(bundle: &ReplayBundle) -> Value {
    let fingerprint = bundle.fingerprint();
    let mut report = json!({"formatVersion":1,"bundleFingerprint":fingerprint,"bundle":bundle,"nodeValidated":false,"status":"incomplete-bundle","scope":"supplied-state and declared-property only; no historical-state or inclusion claim"});
    if bundle.format_version != 1 {
        report["reason"] = json!("unsupported replay format");
        return report;
    }
    let accepted = match validate(&bundle.execution) {
        Ok(a) => a,
        Err(e) => {
            report["status"] = json!(e.status);
            report["execution"] = serde_json::to_value(e).expect("failure serializes");
            return report;
        }
    };
    report["nodeValidated"] = json!(true);
    report["execution"] = accepted.report();
    let records = accepted
        .request()
        .case
        .premises()
        .boxes
        .value()
        .expect("validated state");
    report["inputProvenance"] = json!(if records
        .iter()
        .all(|r| serde_json::to_value(r).expect("record serializes")["origin"] == "source-recorded")
    {
        "source-recorded-inputs"
    } else {
        "hypothetical-or-mixed-inputs"
    });
    report["contextProvenance"] = json!(
        "see explicit per-premise origins; node acceptance does not authenticate state or history"
    );
    match claim::evaluate(&accepted, &bundle.property, fingerprint) {
        Ok(PropertyResult::Violation(claim)) => {
            report["status"] = json!("confirmed-violation");
            report["claim"] = claim.report();
        }
        Ok(PropertyResult::Nonviolating(accounting)) => {
            report["status"] = json!("accepted-nonviolating");
            report["accounting"] = accounting;
        }
        Err(e) => {
            report["status"] = json!("unsupported-property");
            report["reason"] = json!(e);
        }
    }
    report
}
