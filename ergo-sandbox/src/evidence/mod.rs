//! Versioned, unvalidated experiment inputs and provenance-bound static results.
//! Wire construction and explicit supplied-state validation delegate to the pinned node.
pub mod case;
pub mod claim;
pub mod promotion;
pub mod replay;
pub mod sign;
pub mod validate;
pub mod wire;
pub use case::{
    Analysis, BindingSet, CasePremises, ConstantBinding, EvidenceCase, Origin, Premise,
    RecordedBox, SourceIdentity, SourceRecord, StaticAnalysis,
};
