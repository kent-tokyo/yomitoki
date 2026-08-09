//! Provenance construction (AGENTS.md §16, §4.6).

use sha2::{Digest, Sha256};

use crate::config::AnalysisConfig;
use crate::report::Provenance;
use crate::rules::RULESET_VERSION;

/// Bumped whenever the shape of `SynthesizabilityReport` changes in a way
/// that could affect existing consumers.
const SCHEMA_VERSION: &str = "0.1.0";

/// The chematic version requirement rensei is built against (AGENTS.md
/// §4.6). Not read from chematic itself — chematic doesn't expose a
/// version constant — so this is kept in sync with `Cargo.toml` by hand.
const CHEMATIC_VERSION_REQUIREMENT: &str = "0.11";

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
        rensei_version: env!("CARGO_PKG_VERSION").to_string(),
        chematic_version: CHEMATIC_VERSION_REQUIREMENT.to_string(),
        ruleset_version: RULESET_VERSION.to_string(),
        config_hash: config_hash(config),
    }
}
