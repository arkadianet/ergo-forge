//! The audit layer: static lints over the lifted AST.
//!
//! Lints run on the tree the decompiler recovers, so the same lint serves
//! both authored source (compile, then lift) and a contract pasted from
//! chain. See `docs/superpowers/specs/2026-08-31-lift-target-ast-design.md`.

pub mod finding;

pub use finding::{snippet, Finding, Severity, SNIPPET_MAX};
