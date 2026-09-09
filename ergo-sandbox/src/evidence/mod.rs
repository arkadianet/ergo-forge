//! Versioned, unvalidated experiment inputs and provenance-bound static results.
//! This module does not construct canonical transactions or validate execution.
pub mod case;
pub use case::{
    Analysis, BindingSet, CasePremises, ConstantBinding, EvidenceCase, Origin, Premise,
    RecordedBox, SourceIdentity, SourceRecord, StaticAnalysis,
};
