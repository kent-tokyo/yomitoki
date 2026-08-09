//! RENSEI: fast, explainable, route-free molecular synthesizability
//! diagnostics, built on [chematic](https://docs.rs/chematic).
//!
//! See `docs/architecture.md` for the full design contract and current
//! implementation status.

mod analyze;
mod components;
mod config;
mod error;
mod explain;
mod provenance;
mod report;
mod rules;
mod suggestions;

pub use analyze::{analyze, analyze_smiles};
pub use config::{AnalysisConfig, ScoringProfile, Strictness};
pub use error::RenseiError;
pub use report::{
    ApplicabilityReport, AtomIndex, ComponentScore, ComponentScores, ConfidenceScore, Contribution,
    ExpectedEffect, Finding, FindingCode, FindingEvidence, FindingRef, OverallAssessment,
    ProbabilityLikeScore, Provenance, Severity, SimplificationSuggestion, SuggestionCode,
    SynthesizabilityReport, Verdict,
};
