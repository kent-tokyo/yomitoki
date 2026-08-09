//! yomitoki: fast, explainable, route-free molecular synthesizability
//! diagnostics, built on [chematic](https://docs.rs/chematic).
//!
//! See `docs/architecture.md` for the full design contract and current
//! implementation status.
#![warn(missing_docs)]

mod analyze;
mod components;
mod config;
mod error;
mod explain;
mod provenance;
mod report;
mod rules;
mod suggestions;

pub use analyze::{analyze, analyze_batch, analyze_smiles};
pub use config::{AnalysisConfig, ScoringProfile, Strictness};
pub use error::YomitokiError;
pub use report::{
    ApplicabilityReport, AtomIndex, ComponentScore, ComponentScores, ConfidenceScore, Contribution,
    ExpectedEffect, Finding, FindingCode, FindingEvidence, FindingRef, OverallAssessment,
    ProbabilityLikeScore, Provenance, Severity, SimplificationSuggestion, SuggestionCode,
    SynthesizabilityReport, Verdict,
};

/// Re-exported only so `tools/build-fragment-corpus` can filter its input
/// corpus with the exact same element set the library uses (AGENTS.md §11:
/// never let a second tool silently drift from the library's own domain
/// definition). Not part of the supported public API.
#[doc(hidden)]
pub use rules::SUPPORTED_ELEMENTS;
