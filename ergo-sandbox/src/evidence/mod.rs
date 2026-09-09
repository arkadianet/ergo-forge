//! Versioned, unvalidated experiment inputs and provenance-bound static results.
//! Wire construction delegates to the pinned node; execution is not validated.
pub mod case;
pub mod wire;
pub use case::{
    Analysis, BindingSet, CasePremises, ConstantBinding, EvidenceCase, Origin, Premise,
    RecordedBox, SourceIdentity, SourceRecord, StaticAnalysis,
};
