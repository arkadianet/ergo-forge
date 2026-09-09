use std::collections::BTreeMap;
use std::path::Path;
use std::process::Command;

use ergo_sandbox::ingest::{
    ingest_directory, ingest_source, BindingOrigin, IngestOptions, Override, Status,
};
use ergo_sandbox::TypedValue;

fn fixtures() -> &'static Path {
    Path::new(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/ingest"
    ))
}

#[test]
fn usage_fixtures_compile_parse_and_lift_with_expected_bindings() {
    let cases: &[(&str, &[(&str, &str)])] = &[
        (
            "tokens.ergo",
            &[("asset", "Coll[Byte]"), ("quantity", "Long")],
        ),
        (
            "sigma.ergo",
            &[("owner", "SigmaProp"), ("backup", "SigmaProp")],
        ),
        (
            "arithmetic.ergo",
            &[
                ("divisor", "Long"),
                ("minimum", "Long"),
                ("delay", "Int"),
                ("enabled", "Boolean"),
                ("disabled", "Boolean"),
            ],
        ),
        (
            "collections.ergo",
            &[("weights", "Coll[Long]"), ("octets", "Coll[Byte]")],
        ),
        (
            "signatures.ergo",
            &[
                ("script", "Coll[Byte]"),
                ("digest", "Coll[Byte]"),
                ("supplied", "Coll[Byte]"),
            ],
        ),
        ("scopes.ergo", &[("outsideLimit", "Int")]),
        (
            "annotations.ergo",
            &[
                ("declared", "Long"),
                ("threshold", "Long"),
                ("fallback", "Long"),
                ("height", "Int"),
            ],
        ),
    ];
    for (file, expected) in cases {
        let src = std::fs::read_to_string(fixtures().join(file)).unwrap();
        let result = ingest_source(&src, &IngestOptions::default());
        assert_eq!(
            result.report.status,
            Status::Compiled,
            "{file}: {:?}",
            result.report.reason
        );
        let artifact = result.artifact.unwrap();
        assert!(!artifact.tree_bytes.is_empty());
        assert!(!artifact.lifted.truncated);
        assert_eq!(artifact.lifted.raw_placeholders, 0, "{file}");
        let got: BTreeMap<_, _> = result
            .report
            .bindings
            .iter()
            .map(|b| {
                assert_eq!(b.origin, BindingOrigin::Inferred);
                assert!(!b.lines.is_empty());
                (b.parameter.as_str(), b.bound.r#type.as_str())
            })
            .collect();
        assert_eq!(got, expected.iter().copied().collect(), "{file}");
        assert!(!result.report.notes.is_empty());
    }
}

#[test]
fn ambiguous_conflicting_and_unsupported_types_are_named() {
    for (src, name, reason) in [
        ("sigmaProp(mystery == mystery)", "mystery", "ambiguous"),
        ("sigmaProp(items.size > 0)", "items", "ambiguous"),
        ("sigmaProp(items(0) > HEIGHT)", "items", "Coll[Int]"),
        (
            "sigmaProp(SELF.value == wrong && SELF.id == wrong)",
            "wrong",
            "incompatible",
        ),
    ] {
        let result = ingest_source(src, &IngestOptions::default());
        assert_eq!(result.report.status, Status::NotCompiled);
        assert!(result.artifact.is_none());
        let error = result.report.reason.unwrap();
        assert!(error.contains(name) && error.contains(reason), "{error}");
    }
}

#[test]
fn real_values_and_types_override_inference_and_feed_other_constants() {
    let mut options = IngestOptions::default();
    options.overrides.insert(
        "limit".into(),
        Override::Value(TypedValue {
            r#type: "Int".into(),
            value: serde_json::json!(777),
        }),
    );
    let src = "sigmaProp(SELF.value > limit && other == limit)";
    let r = ingest_source(src, &options);
    assert_eq!(r.report.status, Status::Compiled, "{:?}", r.report.reason);
    let limit = &r.report.bindings[0];
    assert_eq!(limit.origin, BindingOrigin::ValueOverride);
    assert_eq!(limit.bound.r#type, "Int");
    assert_eq!(limit.bound.value, serde_json::json!(777));

    options.overrides.clear();
    options.overrides.insert(
        "items".into(),
        Override::Type {
            r#type: "Coll[Byte]".into(),
        },
    );
    let r = ingest_source("sigmaProp(items.size > 0)", &options);
    assert_eq!(r.report.status, Status::Compiled, "{:?}", r.report.reason);
    assert_eq!(r.report.bindings[0].origin, BindingOrigin::TypeOverride);

    options.overrides.insert(
        "items".into(),
        Override::Value(TypedValue {
            r#type: "Long".into(),
            value: serde_json::json!(10),
        }),
    );
    let r = ingest_source("sigmaProp(SELF.id == items)", &options);
    assert_eq!(r.report.status, Status::NotCompiled);
    assert_eq!(r.report.bindings.len(), 1);
    assert_eq!(r.report.bindings[0].origin, BindingOrigin::ValueOverride);
    assert!(r.report.reason.unwrap().contains("items"));
}

#[test]
fn version_gate_is_precise_even_when_version_three_requested() {
    let source = "sigmaProp(unsignedBigInt(\"1\") == SELF.R4[UnsignedBigInt].get)";
    let r = ingest_source(source, &IngestOptions::default());
    assert_eq!(r.report.status, Status::NotCompiled);
    let e = r.report.reason.unwrap();
    assert!(
        e.contains("UnsignedBigInt constant data")
            && e.contains("v0")
            && e.contains("no public compile option"),
        "{e}"
    );
}

#[test]
fn free_occurrence_is_not_hidden_by_a_same_named_local_in_another_scope() {
    let r = ingest_source(
        "{ val check = { val local = 2; local > 0 }; sigmaProp(check && HEIGHT > local) }",
        &IngestOptions::default(),
    );
    assert_eq!(r.report.status, Status::NotCompiled);
    assert_eq!(r.report.bindings[0].parameter, "local");
    let e = r.report.reason.unwrap();
    assert!(
        e.contains("Variable local already defined") && e.contains("bind error"),
        "{e}"
    );
}

#[test]
fn batch_and_cli_keep_failure_rows_and_exit_status() {
    let report = ingest_directory(fixtures(), &IngestOptions::default()).unwrap();
    assert_eq!(report.compiled, 7);
    assert_eq!(report.not_compiled, 0);
    let baseline = IngestOptions {
        infer_constants: false,
        ..Default::default()
    };
    let report = ingest_directory(fixtures(), &baseline).unwrap();
    assert_eq!(report.compiled, 0);
    assert_eq!(report.not_compiled, 7);
    assert!(report.contracts.iter().all(|r| r
        .reason
        .as_ref()
        .unwrap()
        .contains("missing parameters")));
    assert!(report.contracts.windows(2).all(|r| r[0].path < r[1].path));
    let out = Command::new(env!("CARGO_BIN_EXE_ergo-es"))
        .args([
            "ingest",
            fixtures().to_str().unwrap(),
            "--no-infer",
            "--json",
        ])
        .output()
        .unwrap();
    assert!(!out.status.success());
    let json: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(json["not_compiled"], 7);
    assert_eq!(json["contracts"].as_array().unwrap().len(), 7);
    assert!(ingest_directory(&fixtures().join("nonexistent"), &baseline).is_err());
}

#[test]
fn override_json_does_not_turn_malformed_values_into_type_only_overrides() {
    assert!(serde_json::from_str::<Override>(r#"{"type":"Long","unexpected":1}"#).is_err());
    let options = IngestOptions {
        overrides: serde_json::from_str(r#"{"x":{"type":"Long","value":null}}"#).unwrap(),
        ..Default::default()
    };
    let r = ingest_source("sigmaProp(SELF.value > x)", &options);
    assert_eq!(r.report.status, Status::NotCompiled);
    assert!(r.report.reason.unwrap().contains("parameter `x`"));
}

#[test]
fn mixed_batch_keeps_read_errors_and_cli_overrides_work() {
    let dir = std::env::temp_dir().join(format!("ergo-ingest-test-{}", std::process::id()));
    std::fs::create_dir_all(dir.join("nested")).unwrap();
    std::fs::write(dir.join("a.ergo"), "sigmaProp(HEIGHT > 1)").unwrap();
    std::fs::write(dir.join("nested/b.es"), "sigmaProp(xs.size > 0)").unwrap();
    std::fs::write(dir.join("broken.ergo"), [0xff]).unwrap();
    std::fs::write(dir.join("ignored.txt"), "not a contract").unwrap();
    let r = ingest_directory(&dir, &IngestOptions::default()).unwrap();
    assert_eq!((r.compiled, r.not_compiled), (1, 2));
    assert!(r.contracts[1]
        .reason
        .as_ref()
        .unwrap()
        .contains("read error"));
    let params = dir.join("params.json");
    std::fs::write(&params, r#"{"xs":{"type":"Coll[Byte]"}}"#).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_ergo-es"))
        .args([
            "ingest",
            dir.join("nested").to_str().unwrap(),
            "--params",
            params.to_str().unwrap(),
            "--json",
        ])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let json: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(json["compiled"], 1);
    assert_eq!(
        json["contracts"][0]["bindings"][0]["origin"],
        "type_override"
    );
    std::fs::create_dir(dir.join("empty")).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_ergo-es"))
        .args(["ingest", dir.join("empty").to_str().unwrap(), "--json"])
        .output()
        .unwrap();
    assert!(!out.status.success());
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(&dir, dir.join("cycle")).unwrap();
        assert!(ingest_directory(&dir, &IngestOptions::default())
            .unwrap_err()
            .contains("symlink"));
    }
    std::fs::remove_dir_all(dir).unwrap();
}
