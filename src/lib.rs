//! yomitoki: fast, explainable, route-free molecular synthesizability
//! diagnostics, built on [chematic](https://docs.rs/chematic).
//!
//! yomitoki diagnoses **intrinsic structural synthesizability** — burden
//! explainable from the target molecule itself (size, ring topology,
//! stereochemistry, functional-group liability). It does not predict
//! **route-dependent difficulty** (precursor availability, route length,
//! protecting-group strategy, or retrosynthetic search success) — that is
//! a separate, external-context question by design, not current
//! limitation; see [`OverallAssessment`] and `docs/architecture.md`'s
//! "Ecosystem boundary" section.
//!
//! See `docs/architecture.md` for the full design contract and current
//! implementation status.
#![warn(missing_docs)]

mod analyze;
mod components;
mod config;
mod error;
mod explain;
mod fragment_corpus;
mod provenance;
mod report;
mod rules;
mod suggestions;

pub use analyze::{analyze, analyze_batch, analyze_smiles};
pub use config::{AnalysisConfig, FragmentModelConfig, ScoringProfile, Strictness};
pub use error::YomitokiError;
pub use fragment_corpus::FragmentCorpus;
pub use report::{
    ApplicabilityReport, AtomIndex, ComponentScore, ComponentScores, ConfidenceScore, Contribution,
    ExpectedEffect, Finding, FindingCode, FindingEvidence, FindingRef, FragmentCorpusProvenance,
    FragmentPrecedentEvidence, OverallAssessment, ProbabilityLikeScore, Provenance, Severity,
    SimplificationSuggestion, SuggestionCode, SynthesizabilityReport, Verdict,
};

/// Re-exported only so `tools/build-fragment-corpus` can filter its input
/// corpus with the exact same element set the library uses (AGENTS.md §11:
/// never let a second tool silently drift from the library's own domain
/// definition). Not part of the supported public API.
#[doc(hidden)]
pub use rules::SUPPORTED_ELEMENTS;

/// Re-exported only so `tools/build-fragment-corpus` can record which
/// `fragment_precedent` formula/ruleset version was current when a corpus
/// was built (manifest provenance — round 17). Not part of the supported
/// public API.
#[doc(hidden)]
pub use rules::RULESET_VERSION;
