//! W06 is a view of the existing S00/S01 static instruments, not an analyser.
//! Keep the lint's wording, including its limits, and its exact recovery anchor.
use crate::{audit::Audit, checklist::Observation};

pub type Line = Observation;

pub fn negative_space(audit: &Audit) -> Vec<Line> {
    audit
        .findings
        .iter()
        .filter(|finding| {
            matches!(
                finding.lint,
                "unbound-box-reserves"
                    | "unconstrained-outputs"
                    | "successor-field-drift"
                    | "delegated-reserves"
                    | "trust-assumptions"
                    | "trivial-sigma-branch"
                    | "unchecked-get"
                    | "unauthenticated-code-execution"
            ) && crate::checklist::names_lint(finding.lint)
        })
        .map(Observation::from)
        .collect()
}
