//! Authenticate historical fixtures before calling these explicit remeasurement
//! helpers. Production replay still rejects their old revision unchanged.
#![allow(dead_code)]
use ergo_sandbox::evidence::{
    case::engine_revision,
    validate::{validate, ValidationRequest},
    EvidenceCase,
};

pub fn fixture_revision() -> &'static str {
    // Historical provenance, not a second live engine pin.
    include_str!("../fixtures/evidence/manifest.json")
        .rsplit("\"nodeRevision\": \"")
        .next()
        .unwrap()
        .split('"')
        .next()
        .unwrap()
}

pub fn on_current_engine(mut request: ValidationRequest) -> ValidationRequest {
    assert_eq!(request.case.premises().engine_revision, fixture_revision());
    if fixture_revision() != engine_revision() {
        let failure = validate(&request).unwrap_err();
        assert!(
            !failure.pipeline_invoked,
            "historical engine drift must fail closed"
        );
        assert!(!failure.node_validated);
        let original = request.fingerprint();
        let mut premises = request.case.premises().clone();
        premises.engine_revision = engine_revision().into();
        request.case = EvidenceCase::new(premises).unwrap();
        assert_ne!(
            request.fingerprint(),
            original,
            "new pin is a new evidence identity"
        );
    }
    request
}
