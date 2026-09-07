//! `ergo-es eval --json` and `ergo-es test --json`: the machine-readable
//! shape a second reducer (a differential runner) prints to be compared.

use std::io::Write;
use std::process::Command;

fn ergo_es(args: &[&str]) -> (bool, String, String) {
    let out = Command::new(env!("CARGO_BIN_EXE_ergo-es"))
        .args(args)
        .output()
        .expect("run ergo-es");
    (
        out.status.success(),
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

fn tmp(name: &str, content: &str) -> String {
    let p = std::env::temp_dir().join(format!("ergo-es-json-{}-{name}", std::process::id()));
    std::fs::File::create(&p)
        .unwrap()
        .write_all(content.as_bytes())
        .unwrap();
    p.to_string_lossy().into_owned()
}

#[test]
fn eval_json_is_one_object_with_the_verdict_and_cost() {
    let p = tmp(
        "scenario.json",
        r#"{"source":"sigmaProp(HEIGHT > 100)","height":200}"#,
    );
    let (ok, out, err) = ergo_es(&["eval", &p, "--json"]);
    assert!(ok, "stderr: {err}");
    let v: serde_json::Value = serde_json::from_str(&out).expect("one JSON object on stdout");
    assert_eq!(v["verdict"], "pass");
    assert_eq!(v["reducedTo"], "true");
    assert!(v["cost"].as_u64().unwrap() > 0);
    assert!(v["costLimit"].as_u64().unwrap() > 0);
    assert!(v["treeHex"].as_str().unwrap().len() > 4);
    assert!(v["error"].is_null());
}

#[test]
fn test_json_lists_every_case_with_expected_and_actual() {
    let p = tmp(
        "suite.json",
        r#"{"source":"sigmaProp(HEIGHT > 100)","scenarios":[
          {"name":"locked","expect":"fail","height":50},
          {"name":"open","expect":"pass","height":200},
          {"name":"wrong","expect":"pass","height":50}]}"#,
    );
    let (ok, out, _err) = ergo_es(&["test", &p, "--json"]);
    assert!(!ok, "a failing case exits non-zero");
    let v: serde_json::Value = serde_json::from_str(&out).expect("one JSON object on stdout");
    assert_eq!(v["formatVersion"], 1);
    let cases = v["cases"].as_array().unwrap();
    assert_eq!(cases.len(), 3);
    assert_eq!(cases[0]["actual"], "fail");
    assert_eq!(cases[0]["passed"], true);
    assert_eq!(cases[2]["expected"], "pass");
    assert_eq!(cases[2]["actual"], "fail");
    assert_eq!(cases[2]["passed"], false);
    assert_eq!(v["passed"], 2);
    assert_eq!(v["failed"], 1);
    assert!(v["treeHex"].as_str().unwrap().len() > 4);
}
