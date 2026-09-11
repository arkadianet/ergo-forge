mod property_replay_support;
#[allow(dead_code)]
mod property_support;
use ergo_sandbox::properties::{
    replay::{import, replay},
    schema::Declaration,
};
use property_support::*;
use serde_json::{json, Value};
use std::{path::PathBuf, sync::OnceLock};
fn measurement() -> &'static Value {
    static RESULT: OnceLock<Value> = OnceLock::new();
    RESULT.get_or_init(||{
 let (m,e)=inventory();
 let rows:Vec<Value>=m["cases"].as_array().unwrap().iter().map(|r|{
 let started=std::time::Instant::now();
 let c=read(r["path"].as_str().unwrap()); let envelope=property_replay_support::envelope(&c);
 let expected=e["cases"].as_array().unwrap().iter().find(|a|a["id"]==r["id"]).unwrap();
 let outcome=match replay(&envelope) {Ok(result)=>{let imported=import(&serde_json::to_string(&envelope).unwrap(),result.identity()).unwrap(); assert_eq!(result.report(),imported.report()); result.report().clone()},Err(reason)=>json!({"status":if Declaration::parse(&envelope.declaration.to_string()).is_err(){"unsupported-property"}else{"execution-rejected"},"reason":reason})};
 let blocker=if outcome["status"]=="unsupported-property" {json!({"class":"property expressiveness/meaning","evidence":outcome["reason"]})}else if outcome["status"]=="unresolved" {json!({"class":if r["id"]=="short_response_horizon"{"action representation"}else if r["id"]=="arithmetic_overflow"{"operational cap"}else{"property expressiveness/meaning"},"evidence":outcome})}else {json!({"class":"property expressiveness/meaning","evidence":"D00 independent-human-review-missing; consequential meaning and defect attribution unadjudicated"})};
 json!({"id":r["id"],"family":r["family"],"fixtureSha256":r["sha256"],"draftPropertySha256":r["propertySha256"],"envelope":envelope,"identity":envelope.identity(),"expectedDisposition":expected["expectedDisposition"],"outcome":outcome,"earliestEvidencedBlocker":blocker,"secondaryBlockers":["independent-human-review-missing","assumed external root; no validated declared initialization"],"modelReview":"not-adjudicated","contractDefect":"not-adjudicated","attributionExplanation":"A faithfully refuted bad author assertion need not indicate a wrong contract.","authorHours":null,"reviewerHours":null,"replayWallSeconds":started.elapsed().as_secs_f64(),"provenance":"hypothetical","exposure":"author-exposed supplied regression","discoveryCredit":0})
 }).collect();
 let agreement=rows.iter().filter(|r|r["outcome"]["status"]==r["expectedDisposition"]).count();
 let guard_count=rows.iter().filter(|r| r["family"]!="boundary" && r["outcome"]["guard"]==true).count();
 let violations=rows.iter().filter(|r|r["outcome"]["status"]=="violated").count();
 let controls=rows.iter().filter(|r|r["outcome"]["status"]=="holds-on-execution").count();
 let result=json!({"version":"D03-results:v1","guardCoverage":{"numerator":guard_count,"denominator":m["denominators"]["supported"]},"violations":violations,"controls":controls,"manifestSha256":MANIFEST_SHA,"answerSha256":ANSWER_SHA,"cases":rows,"agreement":{"numerator":agreement,"denominator":m["cases"].as_array().unwrap().len()},"transfer":read("transfer-registration.json"),"utility":null,"discoveryCredit":0,"semanticMeasurement":"incomplete-independent-human-review-missing","checkpoint":"D04 not run; first measurement retained for its separate decision"});
 // Only an explicit first-attempt destination writes artifacts; ordinary tests
 // cannot overwrite the original measurement with a retry.
 if let Some(dir)=std::env::var_os("D03_MEASUREMENT_DIR") {
 let dir=PathBuf::from(dir); std::fs::create_dir_all(&dir).unwrap();
 let raw=serde_json::to_vec_pretty(&result).unwrap();
 use std::io::Write;
 std::fs::OpenOptions::new().write(true).create_new(true).open(dir.join("D03-results-raw.fixture")).unwrap().write_all(&raw).unwrap();
 let mut summary=result.clone();
 for r in summary["cases"].as_array_mut().unwrap(){r.as_object_mut().unwrap().remove("envelope");}
 summary["raw"]=json!({"path":"D03-results-raw.fixture","sha256":sha(&raw)});
 let bytes=serde_json::to_vec_pretty(&summary).unwrap();
 std::fs::OpenOptions::new().write(true).create_new(true).open(dir.join("D03-results.json")).unwrap().write_all(&bytes).unwrap();
 println!("D03-results.json sha256={} raw sha256={}",sha(&bytes),sha(&raw));
 }
 println!("D03 agreement={agreement}/24 transfer denominator=null utility=null; no contract defects adjudicated");
 result
 })
}
#[test]
fn all_registered_property_dispositions_match() {
    let r = measurement();
    let (m, _) = inventory();
    assert_eq!(
        r["guardCoverage"]["numerator"],
        m["denominators"]["supported"]
    );
    assert_eq!(r["violations"], m["denominators"]["violations"]);
    assert_eq!(r["controls"], m["denominators"]["controls"]);
    assert_eq!(r["agreement"]["numerator"],r["agreement"]["denominator"],"{}",r["cases"].as_array().unwrap().iter().map(|c|json!({"id":c["id"],"actual":c["outcome"]["status"],"expected":c["expectedDisposition"]})).collect::<Vec<_>>().iter().map(Value::to_string).collect::<Vec<_>>().join("\n"));
}
#[test]
fn registered_author_cases_show_usefulness_beyond_extraction() {
    let r = measurement();
    assert_eq!(r["transfer"]["denominator"],r["transfer"]["plannedCases"],"Frozen transfer registration is ungrounded; utility remains null. Synthetic agreement cannot substitute.");
    assert_eq!(r["transfer"]["utilityPass"], true);
}
#[test]
fn property_replay_rejects_tampered_claims_and_premises() {
    let c = read("cases/reserve_violation_1.fixture");
    let e = property_replay_support::envelope(&c);
    let id = e.identity();
    let v = serde_json::to_value(&e).unwrap();
    for key in v.as_object().unwrap().keys() {
        let mut bad = v.clone();
        bad[key] = Value::Null;
        assert!(import(&bad.to_string(), &id).is_err());
    }
    for key in ["violated", "status", "report", "accepted"] {
        let mut bad = v.clone();
        bad[key] = json!(true);
        assert!(import(&bad.to_string(), &id).is_err());
    }
    let mut bad = e.clone();
    bad.declaration["assertion"] = json!({"op":"boolean","value":true});
    assert_ne!(id, bad.identity());
    assert!(import(&serde_json::to_string(&bad).unwrap(), &id).is_err());
    assert_eq!(
        replay(&bad).unwrap().report()["status"],
        "holds-on-execution"
    );
    let mut bad = e.clone();
    bad.steps[0].transaction_bytes = "00".into();
    assert!(replay(&bad).is_err());
    let mut bad = e.clone();
    bad.root.clear();
    assert!(replay(&bad).is_err());
    let mut bad = e;
    bad.node_revision = "unknown".into();
    assert!(replay(&bad).is_err());
}
#[test]
fn legacy_replay_bytes_and_semantics_remain_unchanged() {
    let (m, _) = inventory();
    let current = measure(&m);
    let frozen = read("legacy-results-raw.fixture");
    assert_eq!(current, frozen);
}
