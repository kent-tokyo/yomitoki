//! Report schema. See `docs/architecture.md` for the full contract.

use serde::{Deserialize, Serialize};

/// Clamp to `0.0..=1.0`, mapping non-finite input to `0.0`.
///
/// The single chokepoint every score-like value passes through, so "never
/// NaN, never out of range" (AGENTS.md §16, §29) holds everywhere at once
/// instead of being checked ad hoc at each call site.
fn clamp01(value: f64) -> f64 {
    if value.is_finite() {
        value.clamp(0.0, 1.0)
    } else {
        0.0
    }
}

/// Map non-finite values to `0.0`, otherwise pass through unchanged.
///
/// Used for fields that are meaningful outside `0.0..=1.0` (e.g. raw,
/// unnormalized burden sums) but must still never be NaN/Infinity in
/// serialized output.
pub(crate) fn finite_or_zero(value: f64) -> f64 {
    if value.is_finite() { value } else { 0.0 }
}

/// A score in `0.0..=1.0` describing how synthesizable/difficult a molecule
/// is judged to be. `1.0` = maximally synthesizable / no difficulty burden,
/// depending on which field it appears in — see `OverallAssessment`.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ProbabilityLikeScore(f64);

impl ProbabilityLikeScore {
    pub fn new(value: f64) -> Self {
        Self(clamp01(value))
    }

    pub fn value(&self) -> f64 {
        self.0
    }
}

/// A score in `0.0..=1.0` describing how reliable an assessment is. Kept as
/// a distinct type from [`ProbabilityLikeScore`] even though the underlying
/// representation is identical, because AGENTS.md §6 treats confidence and
/// score as semantically separate fields that must never be conflated.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ConfidenceScore(f64);

impl ConfidenceScore {
    pub fn new(value: f64) -> Self {
        Self(clamp01(value))
    }

    pub fn value(&self) -> f64 {
        self.0
    }
}

/// A rensei-owned atom index, decoupled from chematic's internal `AtomIdx`
/// representation (AGENTS.md §11: don't over-couple the public API to
/// chematic internals).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct AtomIndex(pub u32);

impl From<chematic::core::AtomIdx> for AtomIndex {
    fn from(idx: chematic::core::AtomIdx) -> Self {
        AtomIndex(idx.0)
    }
}

/// Machine-readable finding code (AGENTS.md §8.1). Only variants the
/// currently-implemented components emit exist so far; codes for
/// not-yet-implemented components are added alongside those components.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum FindingCode {
    RingBridgedComplexity,
    RingSpiro,
    RingFusedDense,
    RingMacrocycle,
    SizeLargeMolecularWeight,
    SizeHighRotatableBondCount,
    StereoCenterCount,
    StereoDensityHigh,
    InputUnsupportedElement,
    InputDisconnected,
    InputUnusualValence,
    InputTooLarge,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Severity {
    Low,
    Medium,
    High,
}

/// Structured, numeric evidence backing a [`Finding`] (AGENTS.md §8.2).
/// Fields are `None` when not applicable to a given finding code.
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
pub struct FindingEvidence {
    pub value: Option<f64>,
    pub threshold: Option<f64>,
}

/// One diagnostic finding. `explanation` is generated from `code` +
/// `evidence` by `explain::render` — never authored by hand per instance
/// (AGENTS.md §8.3: structured data is the source of truth).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Finding {
    pub code: FindingCode,
    pub severity: Severity,
    pub confidence: ProbabilityLikeScore,
    pub atoms: Vec<AtomIndex>,
    pub evidence: FindingEvidence,
    pub explanation: String,
}

/// Index into `SynthesizabilityReport.findings`, used by
/// `ComponentScore.findings` to reference findings without duplicating them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct FindingRef(pub usize);

/// A named factor's contribution to the overall assessment, used in
/// `dominant_penalties`/`dominant_supports` (AGENTS.md §4.1).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Contribution {
    pub code: FindingCode,
    pub name: String,
    pub contribution: ProbabilityLikeScore,
}

/// One component's score, in isolation, before aggregation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ComponentScore {
    /// Unnormalized burden; meaningful for debugging/tuning, not bounded to
    /// `0.0..=1.0`.
    pub raw: f64,
    pub normalized: ProbabilityLikeScore,
    pub confidence: ProbabilityLikeScore,
    pub contribution: ProbabilityLikeScore,
    pub findings: Vec<FindingRef>,
}

/// Per-component scores. Each field is `Option` — `None` means "not
/// evaluated in this version," not "evaluated, found nothing" (a fabricated
/// zero would be dishonest). Only `ring_topology` and `input_quality` are
/// `Some` in v0.1; see `docs/architecture.md`.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ComponentScores {
    pub size_topology: Option<ComponentScore>,
    pub ring_topology: Option<ComponentScore>,
    pub stereochemical_burden: Option<ComponentScore>,
    pub fragment_rarity: Option<ComponentScore>,
    pub functional_group_liability: Option<ComponentScore>,
    pub input_quality: Option<ComponentScore>,
}

/// AGENTS.md §7. All six variants exist for schema stability even though
/// only a subset is reachable with today's two implemented components (see
/// `docs/architecture.md`).
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Verdict {
    LikelyAccessible,
    ModeratelyAccessible,
    Challenging,
    HighlyChallenging,
    Indeterminate,
    OutOfDomain,
}

/// AGENTS.md §6. `synthesizability`/`difficulty` are complementary in v0.1
/// as an implementation choice, not a permanent API guarantee.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OverallAssessment {
    pub synthesizability: ProbabilityLikeScore,
    pub difficulty: ProbabilityLikeScore,
    pub confidence: ConfidenceScore,
    pub verdict: Verdict,
}

/// AGENTS.md §5.6. Kept distinct from `OverallAssessment` so applicability
/// is never conflated with score or confidence.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ApplicabilityReport {
    pub supported_elements: bool,
    pub sanitized: bool,
    pub stereo_complete: bool,
    pub disconnected: bool,
    pub unusual_valence: bool,
    /// Distance from the calibration corpus. Always `None` until a
    /// calibration corpus exists (Phase 2+).
    pub domain_distance: Option<f64>,
}

/// AGENTS.md §9. Heuristic, never a guarantee that a change will help.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SuggestionCode {
    ReduceStereocenterDensity,
    ReplaceBridgedRingWithMonocyclicAnalog,
    ReduceAdjacentQuaternaryCenters,
    RemoveSimilarReactiveGroup,
    IncreaseFragmentPrecedent,
    SimplifyMacrocyclicClosure,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ExpectedEffect {
    LikelyReducesDifficulty,
    MayReduceDifficulty,
    Uncertain,
}

/// AGENTS.md §9. No component in v0.1 produces these yet; the schema field
/// exists so `suggestions` is additive-safe once one does.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SimplificationSuggestion {
    pub code: SuggestionCode,
    pub target_atoms: Vec<AtomIndex>,
    pub rationale: String,
    pub expected_effect: ExpectedEffect,
    pub confidence: ProbabilityLikeScore,
}

/// AGENTS.md §16.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Provenance {
    pub schema_version: String,
    pub rensei_version: String,
    pub chematic_version: String,
    pub ruleset_version: String,
    pub config_hash: String,
}

/// The top-level report returned by `analyze`/`analyze_smiles`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SynthesizabilityReport {
    pub overall: OverallAssessment,
    pub components: ComponentScores,
    pub findings: Vec<Finding>,
    pub dominant_penalties: Vec<Contribution>,
    pub dominant_supports: Vec<Contribution>,
    /// Always empty in v0.1 — no component produces suggestions yet.
    pub suggestions: Vec<SimplificationSuggestion>,
    pub applicability: ApplicabilityReport,
    pub provenance: Provenance,
}

impl SynthesizabilityReport {
    /// Findings sorted by contribution magnitude, highest first — the
    /// "Dominant penalties" list in AGENTS.md §4.1/§15.
    pub fn dominant_penalties(&self) -> &[Contribution] {
        &self.dominant_penalties
    }
}
