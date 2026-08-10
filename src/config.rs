//! Analysis configuration. See `docs/architecture.md`.

use std::sync::Arc;

use serde::Serialize;

use crate::fragment_corpus::FragmentCorpus;

/// Scoring profile. Only `GeneralOrganic` is implemented — AGENTS.md §12
/// explicitly forbids publishing unimplemented profiles as dummies, so no
/// `MedicinalChemistry`/`Custom` placeholder variants exist yet.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize)]
pub enum ScoringProfile {
    /// General organic chemistry — the only profile implemented in v0.1.
    #[default]
    GeneralOrganic,
}

/// How aggressively borderline molecules are pushed toward abstention
/// (`Verdict::Indeterminate`/`Verdict::OutOfDomain`). Affects the
/// applicability component's confidence-penalty weighting.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize)]
pub enum Strictness {
    /// Lowest confidence threshold for `Verdict::Indeterminate` — most
    /// tolerant of low-confidence input.
    Lenient,
    /// The default threshold.
    #[default]
    Standard,
    /// Highest confidence threshold — abstains most readily.
    Strict,
}

/// Fragment model configuration (AGENTS.md §12's `fragment_model` field).
/// `corpus: None` (the default) disables the `fragment_precedent`
/// component entirely — `SynthesizabilityReport.fragment_precedent` stays
/// `None` (round 21 moved this field out of `ComponentScores`, since the
/// signal no longer contributes to `overall.difficulty` — see `rules.rs`'s
/// "Fragment precedent" section), the same default-off behavior as every
/// v0.1 release. No corpus ships with yomitoki itself (AGENTS.md §5.4
/// forbids embedding one directly in the library); load one with
/// [`FragmentCorpus::load_dir`] and attach it here to enable the component.
///
/// Named `FragmentModelConfig`/`fragment_model`, not
/// `FragmentPrecedentConfig`/`fragment_precedent` (considered and rejected
/// in round 18, alongside the `fragment_rarity` -> `fragment_precedent`
/// rename elsewhere in this crate): this type is already the generic
/// "config for whatever fragment-level model is configured," not tied to
/// the precedent-scoring approach specifically — it currently holds
/// exactly one thing (a `FragmentCorpus`), but the name deliberately
/// leaves room for a future fragment-level model that isn't
/// precedent-based without implying a mismatch the way a
/// precedent-specific name would.
///
/// `Serialize` is hand-implemented (not derived) so `AnalysisConfig`'s
/// `config_hash` reflects *which* corpus is configured (via
/// `FragmentCorpus::version`) without serializing the corpus's full
/// fragment table on every hash computation — a loaded corpus can be
/// several megabytes; its identity, not its content, is what needs to be
/// distinguishable across configs.
#[non_exhaustive]
#[derive(Debug, Clone, Default, PartialEq)]
pub struct FragmentModelConfig {
    /// A loaded fragment-frequency corpus. `None` by default.
    pub corpus: Option<Arc<FragmentCorpus>>,
}

impl Serialize for FragmentModelConfig {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeStruct;
        let mut state = serializer.serialize_struct("FragmentModelConfig", 1)?;
        state.serialize_field(
            "corpus_version",
            &self.corpus.as_deref().map(FragmentCorpus::version),
        )?;
        state.end()
    }
}

/// Analysis configuration. `#[non_exhaustive]` so new fields (e.g.
/// `abstention_policy` from AGENTS.md §12's full sketch) can be added later
/// without breaking existing callers — they're omitted for now rather than
/// included as inert placeholders, since nothing reads them yet.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct AnalysisConfig {
    /// Which scoring profile to use — see [`ScoringProfile`].
    pub profile: ScoringProfile,
    /// How readily to abstain (`Verdict::Indeterminate`) on low-confidence
    /// input — see [`Strictness`].
    pub strictness: Strictness,
    /// Heavy-atom count above which a molecule is `Verdict::OutOfDomain`
    /// (`FindingCode::InputTooLarge`). Defaults to 150.
    pub max_heavy_atoms: usize,
    /// Fragment-precedent corpus configuration — see [`FragmentModelConfig`].
    pub fragment_model: FragmentModelConfig,
}

impl Default for AnalysisConfig {
    fn default() -> Self {
        Self {
            profile: ScoringProfile::default(),
            strictness: Strictness::default(),
            max_heavy_atoms: 150,
            fragment_model: FragmentModelConfig::default(),
        }
    }
}
