//! Provenance construction (AGENTS.md §16, §4.6).

use sha2::{Digest, Sha256};

use crate::config::AnalysisConfig;
use crate::report::{FragmentCorpusProvenance, Provenance};
use crate::rules::RULESET_VERSION;

/// Bumped whenever the shape of `SynthesizabilityReport` changes in a way
/// that could affect existing consumers. `0.6.0` (round 21): `fragment_
/// precedent` moved out of `ComponentScores` (which now only holds
/// difficulty-contributing components) to a new top-level
/// `SynthesizabilityReport.fragment_precedent: Option<FragmentPrecedentEvidence>`
/// field — option C, see `rules.rs`'s "Fragment precedent" section.
const SCHEMA_VERSION: &str = "0.6.0";

/// The chematic version requirement yomitoki is built against (AGENTS.md
/// §4.6). Not read from chematic itself — chematic doesn't expose a
/// version constant — so this is kept in sync with `Cargo.toml` by hand.
const CHEMATIC_VERSION_REQUIREMENT: &str = "0.12";

/// Deterministic config fingerprint: SHA-256 over the config's canonical
/// JSON serialization. Not `std::hash::DefaultHasher` — that hasher is
/// process-randomized on recent Rust versions and would silently break
/// both determinism (§4.5) and cross-run provenance comparability (§4.6).
fn config_hash(config: &AnalysisConfig) -> String {
    let canonical = serde_json::to_vec(config).expect("AnalysisConfig serialization is infallible");
    let digest = Sha256::digest(&canonical);
    format!("sha256:{digest:x}")
}

pub(crate) fn build(config: &AnalysisConfig) -> Provenance {
    Provenance {
        schema_version: SCHEMA_VERSION.to_string(),
        yomitoki_version: env!("CARGO_PKG_VERSION").to_string(),
        chematic_version: CHEMATIC_VERSION_REQUIREMENT.to_string(),
        ruleset_version: RULESET_VERSION.to_string(),
        fragment_corpus: config.fragment_model.corpus.as_deref().map(|corpus| {
            FragmentCorpusProvenance {
                version: corpus.version().to_string(),
                source_name: corpus.domain.source_name.clone(),
                domain: corpus.domain.domain.clone(),
                synthesis_focused: corpus.domain.synthesis_focused,
                description: corpus.domain.description.clone(),
                fragment_definition_version: corpus.fragment_definition_version.clone(),
                reference_distribution_version: corpus.reference_distribution_version.clone(),
            }
        }),
        config_hash: config_hash(config),
    }
}
