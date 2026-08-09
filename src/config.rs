//! Analysis configuration. See `docs/architecture.md`.

use serde::Serialize;

/// Scoring profile. Only `GeneralOrganic` is implemented — AGENTS.md §12
/// explicitly forbids publishing unimplemented profiles as dummies, so no
/// `MedicinalChemistry`/`Custom` placeholder variants exist yet.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize)]
pub enum ScoringProfile {
    #[default]
    GeneralOrganic,
}

/// How aggressively borderline molecules are pushed toward abstention
/// (`Verdict::Indeterminate`/`Verdict::OutOfDomain`). Affects the
/// applicability component's confidence-penalty weighting.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize)]
pub enum Strictness {
    Lenient,
    #[default]
    Standard,
    Strict,
}

/// Analysis configuration. `#[non_exhaustive]` so new fields (e.g.
/// `fragment_model`, `abstention_policy` from AGENTS.md §12's full sketch)
/// can be added later without breaking existing callers — they're omitted
/// for now rather than included as inert placeholders, since nothing reads
/// them yet.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct AnalysisConfig {
    pub profile: ScoringProfile,
    pub strictness: Strictness,
    pub max_heavy_atoms: usize,
}

impl Default for AnalysisConfig {
    fn default() -> Self {
        Self {
            profile: ScoringProfile::default(),
            strictness: Strictness::default(),
            max_heavy_atoms: 150,
        }
    }
}
