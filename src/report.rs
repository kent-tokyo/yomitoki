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
    /// Clamp `value` into `0.0..=1.0`, mapping non-finite input to `0.0`.
    pub fn new(value: f64) -> Self {
        Self(clamp01(value))
    }

    /// The underlying `0.0..=1.0` value.
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
    /// Clamp `value` into `0.0..=1.0`, mapping non-finite input to `0.0`.
    pub fn new(value: f64) -> Self {
        Self(clamp01(value))
    }

    /// The underlying `0.0..=1.0` value.
    pub fn value(&self) -> f64 {
        self.0
    }
}

/// A yomitoki-owned atom index, decoupled from chematic's internal `AtomIdx`
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
    /// A bridged polycyclic ring system (bridgehead connectivity).
    RingBridgedComplexity,
    /// A spiro ring junction (two rings sharing exactly one atom).
    RingSpiro,
    /// A fused ring system whose fusion density is above the threshold.
    RingFusedDense,
    /// A ring at or above the macrocycle size threshold.
    RingMacrocycle,
    /// Molecular weight above the size threshold.
    SizeLargeMolecularWeight,
    /// Rotatable bond count above the threshold.
    SizeHighRotatableBondCount,
    /// At least one tetrahedral stereocenter is present (specified or
    /// unspecified — see `evidence.value` for the count).
    StereoCenterCount,
    /// Stereocenter density (centers per heavy atom) above the threshold.
    StereoDensityHigh,
    /// Stereo analysis could not be run at all for this molecule. Not
    /// reachable as of chematic 0.13.0 — the one trigger condition
    /// (a negatively charged atom, [chematic#267](https://github.com/kent-tokyo/chematic/issues/267))
    /// was an upstream bug, fixed directly rather than worked around.
    /// Kept in the schema for compatibility (`#[non_exhaustive]`), same
    /// policy as this project's other retired-but-not-removed codes.
    StereoAnalysisSkipped,
    /// A Brenk et al. (2008) reactive/unstable structural alert matched —
    /// the specific alert name is in `explanation`, not a separate code
    /// (AGENTS.md §8.1: one generic code per component concern, not one per
    /// pattern).
    FunctionalGroupReactive,
    /// Distinct functional-group cluster count (Ertl 2017) above the
    /// threshold.
    FunctionalGroupDense,
    /// This molecule's fragments sit at a low percentile of the configured
    /// [`crate::FragmentCorpus`]'s (`AnalysisConfig.fragment_model`) own
    /// mean-document-frequency distribution — i.e. this molecule's
    /// fragments are, relative to the reference corpus, weakly precedented
    /// (unusually rare). Only ever produced when a corpus is configured.
    /// **Explanatory only since round 21 (option C)** — does not
    /// contribute to `overall.difficulty` and never appears in
    /// `dominant_penalties`/`dominant_supports`; see
    /// [`FragmentPrecedentEvidence`] and `rules.rs`'s "Fragment precedent"
    /// section for why (round 20 found the corpus-relative signal too
    /// corpus-sensitive to trust as a scoring input). The opposite
    /// -direction case is [`FindingCode::FragmentPrecedentStrong`].
    FragmentPrecedentWeak,
    /// This molecule's fragments sit at a high percentile of the
    /// configured [`crate::FragmentCorpus`]'s own mean-document-frequency
    /// distribution — unusually strong precedent relative to the
    /// reference corpus. Only ever produced when a corpus is configured.
    /// **Explanatory only since round 21 (option C)** — same as
    /// [`FindingCode::FragmentPrecedentWeak`], does not contribute to
    /// `overall.difficulty` and never appears in `dominant_penalties`/
    /// `dominant_supports`.
    FragmentPrecedentStrong,
    /// The molecule contains an element outside yomitoki's supported set.
    InputUnsupportedElement,
    /// The molecule consists of disconnected fragments.
    InputDisconnected,
    /// At least one atom has a valence outside normal ranges for its
    /// element.
    InputUnusualValence,
    /// Heavy-atom count exceeds `AnalysisConfig.max_heavy_atoms`.
    InputTooLarge,
}

/// Author-assigned finding severity. A schema field today, not yet a
/// calibrated signal — see `docs/architecture.md`'s Confidence contract.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Severity {
    /// Minor concern.
    Low,
    /// Moderate concern.
    Medium,
    /// Major concern.
    High,
}

/// Structured, numeric evidence backing a [`Finding`] (AGENTS.md §8.2).
/// Fields are `None` when not applicable to a given finding code.
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
pub struct FindingEvidence {
    /// The measured value this finding is based on (e.g. a ring size, a
    /// stereocenter count, a molecular weight).
    pub value: Option<f64>,
    /// The threshold `value` was compared against, if this finding is
    /// threshold-triggered.
    pub threshold: Option<f64>,
}

/// One diagnostic finding. `explanation` is generated from `code` +
/// `evidence` by `explain::render` — never authored by hand per instance
/// (AGENTS.md §8.3: structured data is the source of truth).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Finding {
    /// Machine-readable identifier for this finding.
    pub code: FindingCode,
    /// Author-assigned severity — see [`Severity`].
    pub severity: Severity,
    /// How certain this specific finding is (distinct from the molecule's
    /// overall `confidence` — see AGENTS.md §6).
    pub confidence: ProbabilityLikeScore,
    /// Atom indices this finding is about. Empty for molecule-level
    /// findings not tied to one specific region (e.g. density-based ones).
    pub atoms: Vec<AtomIndex>,
    /// Structured numeric evidence backing this finding.
    pub evidence: FindingEvidence,
    /// Human-readable explanation, generated from `code` and `evidence`.
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
    /// The finding code this contribution corresponds to.
    pub code: FindingCode,
    /// Human-readable name for this contribution (the finding's own
    /// `explanation` text, so ranking lists stay self-explanatory without a
    /// separate lookup).
    pub name: String,
    /// This contribution's actual weight toward `overall.difficulty` — the
    /// basis `dominant_penalties`/`dominant_supports` are ranked by, not
    /// `Finding.severity` (independent axes; see `docs/architecture.md`).
    pub contribution: ProbabilityLikeScore,
}

/// One component's score, in isolation, before aggregation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ComponentScore {
    /// Unnormalized burden; meaningful for debugging/tuning, not bounded to
    /// `0.0..=1.0`.
    pub raw: f64,
    /// `raw` passed through the non-linear burden transform (AGENTS.md
    /// §5.1) — the value actually used in aggregation.
    pub normalized: ProbabilityLikeScore,
    /// How reliable this component's own score is, independent of the
    /// molecule's overall `confidence` (which today comes entirely from
    /// `input_quality` — see `docs/architecture.md`'s Confidence contract).
    pub confidence: ProbabilityLikeScore,
    /// This component's actual contribution to `overall.difficulty` — a
    /// plain signed `f64`, not `ProbabilityLikeScore`. Always non-negative
    /// for every component in `ComponentScores` today (each argues
    /// difficulty should only be *higher*); kept signed rather than
    /// re-typed to `ProbabilityLikeScore` because it was originally sized
    /// for `fragment_precedent`'s precedent-support case, which *could*
    /// contribute a negative value before round 21 moved that component
    /// out of `ComponentScores` entirely (option C — see `rules.rs`'s
    /// "Fragment precedent" section) — a future component with genuine
    /// support-flavored evidence would need this signedness again.
    pub contribution: f64,
    /// References into `SynthesizabilityReport.findings` for the findings
    /// that justify this component's score.
    pub findings: Vec<FindingRef>,
}

/// Per-component scores that feed `overall.difficulty`. Each field is
/// `Option` — `None` means "not evaluated in this version," not "evaluated,
/// found nothing" (a fabricated zero would be dishonest); every field here
/// is always `Some`. `fragment_precedent` does **not** appear in this
/// struct (round 21 / option C) — its signal doesn't contribute to
/// `overall.difficulty`, so it doesn't belong among components that do; see
/// [`SynthesizabilityReport::fragment_precedent`] for where it now lives.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ComponentScores {
    /// The `size_topology` component's score (molecular weight, rotatable
    /// bond count).
    pub size_topology: Option<ComponentScore>,
    /// The `ring_topology` component's score (ring-shape complexity).
    pub ring_topology: Option<ComponentScore>,
    /// The `stereochemical_burden` component's score (tetrahedral
    /// stereocenter count and density).
    pub stereochemical_burden: Option<ComponentScore>,
    /// The `functional_group_liability` component's score (reactive/
    /// unstable functional groups, dense functionalization).
    pub functional_group_liability: Option<ComponentScore>,
    /// The `input_quality`/applicability component's score.
    pub input_quality: Option<ComponentScore>,
}

/// AGENTS.md §7. All six variants exist for schema stability even though
/// only a subset is reachable with today's two implemented components (see
/// `docs/architecture.md`).
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Verdict {
    /// Low difficulty, high confidence.
    LikelyAccessible,
    /// Moderate difficulty.
    ModeratelyAccessible,
    /// High difficulty.
    Challenging,
    /// Very high difficulty.
    HighlyChallenging,
    /// Confidence too low to bucket by difficulty at all (see
    /// `docs/architecture.md`'s Abstention contract) — not the same as
    /// `OutOfDomain`, which is a hard structural trigger.
    Indeterminate,
    /// A hard applicability trigger fired (unsupported element,
    /// disconnected input, or exceeds the configured size limit) — the
    /// molecule is outside yomitoki's v0.1 scope, not merely low-confidence.
    OutOfDomain,
}

/// AGENTS.md §6. `synthesizability`/`difficulty` are complementary in v0.1
/// as an implementation choice, not a permanent API guarantee.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OverallAssessment {
    /// `1.0 - difficulty.value()` in v0.1.
    pub synthesizability: ProbabilityLikeScore,
    /// The weighted-sum aggregate of every implemented difficulty-
    /// contributing component's `normalized` score.
    pub difficulty: ProbabilityLikeScore,
    /// How reliable this assessment is — from `input_quality`/applicability
    /// only in v0.1, never conflated with `difficulty` (AGENTS.md §6).
    pub confidence: ConfidenceScore,
    /// The bucketed verdict — see [`Verdict`].
    pub verdict: Verdict,
}

/// AGENTS.md §5.6. Kept distinct from `OverallAssessment` so applicability
/// is never conflated with score or confidence.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ApplicabilityReport {
    /// `false` if any atom's element is outside yomitoki's supported set —
    /// a hard `OutOfDomain` trigger.
    pub supported_elements: bool,
    /// `false` if `validate_valence` reported any violation.
    pub sanitized: bool,
    /// `false` if at least one tetrahedral stereocenter is unspecified —
    /// or if stereo analysis couldn't run at all, see `stereo_uncheckable`.
    pub stereo_complete: bool,
    /// `true` when stereo analysis (`stereo_complete`, and
    /// `stereochemical_burden`'s own score) could not be run at all, rather
    /// than genuinely finding zero/incomplete stereocenters. Always
    /// `false` as of chematic 0.13.0 — the one trigger condition (a
    /// negatively charged atom, which used to hit an arithmetic-overflow
    /// bug in chematic's Morgan-rank computation, filed upstream as
    /// chematic issue #267) was fixed directly upstream rather than
    /// worked around, so this field has no remaining trigger. Kept in the
    /// schema (never fabricates `stereo_complete=true` in its place)
    /// rather than removed, in case a genuinely new uncheckable condition
    /// is found later. Distinct from `stereo_complete` on purpose: "we
    /// checked and some centers are unspecified" and "we could not check
    /// at all" are different findings that call for different actions
    /// from a report reader.
    pub stereo_uncheckable: bool,
    /// `true` if the molecule consists of disconnected fragments — a hard
    /// `OutOfDomain` trigger.
    pub disconnected: bool,
    /// `true` if any atom has a valence outside normal ranges for its
    /// element — a soft confidence penalty, not a hard trigger.
    pub unusual_valence: bool,
    /// Distance from the calibration corpus. Always `None` until a
    /// calibration corpus exists (Phase 2+).
    pub domain_distance: Option<f64>,
}

/// AGENTS.md §9. Heuristic, never a guarantee that a change will help.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SuggestionCode {
    /// Reduce the number/concentration of stereocenters.
    ReduceStereocenterDensity,
    /// Replace a bridged ring system with a monocyclic analog.
    ReplaceBridgedRingWithMonocyclicAnalog,
    /// Reduce adjacent quaternary stereocenters — not yet reachable in
    /// v0.1 (no quaternary-adjacency detector exists; see
    /// `docs/architecture.md`).
    ReduceAdjacentQuaternaryCenters,
    /// Remove one of several similar reactive functional groups — not yet
    /// reachable in v0.1 (`brenk_matches_detailed` unions atoms per
    /// pattern, not per occurrence; see `docs/architecture.md`).
    RemoveSimilarReactiveGroup,
    /// Favor a more precedented/common fragment. **Not reachable since
    /// round 21** — kept for schema stability (`SuggestionCode` is
    /// `#[non_exhaustive]`), not emitted: once `fragment_precedent` no
    /// longer contributes to `overall.difficulty` (option C), "increase
    /// precedent" can no longer be truthfully described as
    /// `MayReduceDifficulty` the way this suggestion type requires — see
    /// `suggestions.rs` and `rules.rs`'s "Fragment precedent" section.
    IncreaseFragmentPrecedent,
    /// Simplify a macrocyclic ring closure.
    SimplifyMacrocyclicClosure,
}

/// How likely a [`SimplificationSuggestion`] is to actually reduce
/// difficulty, if followed. Every v0.1 suggestion uses
/// `MayReduceDifficulty` — nothing is calibrated yet, so
/// `LikelyReducesDifficulty` is never emitted.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ExpectedEffect {
    /// Calibrated evidence the change would help — not used in v0.1.
    LikelyReducesDifficulty,
    /// A structural heuristic suggests the change would help, unvalidated.
    MayReduceDifficulty,
    /// No basis to predict direction.
    Uncertain,
}

/// AGENTS.md §9. Heuristic and diagnostic-only — derived from findings that
/// already exist, not from actually rewriting the structure. `confidence`
/// is deliberately flat across every v0.1 suggestion (see
/// `rules::SUGGESTION_CONFIDENCE_HEURISTIC`), since none of them are
/// calibrated against real synthesis outcomes.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SimplificationSuggestion {
    /// Machine-readable identifier for this suggestion.
    pub code: SuggestionCode,
    /// Atom indices this suggestion targets, copied from the source
    /// finding's own `atoms` — may be empty if that finding never carries
    /// atom indices (e.g. density-based ones).
    pub target_atoms: Vec<AtomIndex>,
    /// Human-readable rationale, generated from the source finding.
    pub rationale: String,
    /// How likely this suggestion is to help, if followed.
    pub expected_effect: ExpectedEffect,
    /// How confident this suggestion is — a flat, named constant across
    /// every v0.1 suggestion, not a calibrated per-suggestion value.
    pub confidence: ProbabilityLikeScore,
}

/// AGENTS.md §16.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Provenance {
    /// Version of the report schema itself (independent of `yomitoki_version`).
    pub schema_version: String,
    /// The yomitoki crate version that produced this report.
    pub yomitoki_version: String,
    /// The chematic version dependency requirement.
    pub chematic_version: String,
    /// Version of the named thresholds/weights in `rules.rs` — bumped
    /// whenever any scoring constant changes.
    pub ruleset_version: String,
    /// Provenance for the configured fragment-precedent reference corpus,
    /// or `None` if `AnalysisConfig.fragment_model` has no corpus
    /// configured (the default — no corpus ships with yomitoki itself;
    /// AGENTS.md §5.4). Round 18 replaced the AGENTS.md §16 sketch's
    /// always-populated `model_version: String` with this richer,
    /// still-`Option` structure: a bare version string can't answer "which
    /// chemical domain does this precedent signal actually reflect" — see
    /// [`FragmentCorpusProvenance`].
    pub fragment_corpus: Option<FragmentCorpusProvenance>,
    /// SHA-256 of the `AnalysisConfig`'s canonical JSON serialization, so
    /// reports produced under different configs are distinguishable.
    pub config_hash: String,
}

/// Provenance for the fragment-precedent reference corpus a report was
/// produced under (AGENTS.md §5.4; round 18). Exists so a report can be
/// traced back to which corpus — and which chemical domain — actually
/// produced its `fragment_precedent` signal: "rare in ChEMBL" and "hard to
/// synthesize" are not the same claim (see `rules.rs`'s "Fragment
/// precedent" section), and a report reader needs the corpus's own domain
/// declaration to tell them apart. This is a provenance declaration, not a
/// correctness guarantee, and (deliberately, this round) not yet wired
/// into scoring or confidence — see `synthesis_focused`'s own doc.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FragmentCorpusProvenance {
    /// The corpus's own identifier (`FragmentCorpus::version`, currently
    /// its `artifact_sha256`) — what `Provenance.model_version` held
    /// before round 18.
    pub version: String,
    /// Human-readable name of the corpus's source (e.g. `"ChEMBL 37"`).
    pub source_name: String,
    /// What chemical space this corpus is claimed to represent (e.g.
    /// `"bioactivity"`) — free text, not a closed enum, since the set of
    /// domains a future corpus might declare isn't known in advance.
    pub domain: String,
    /// Whether this corpus's builder claims it represents synthetic
    /// precedent specifically, as opposed to (e.g.) bioactivity-screening
    /// prevalence. A provenance declaration copied from the corpus
    /// manifest, not something yomitoki verifies — and, this round,
    /// deliberately not wired into any scoring/confidence behavior (a
    /// `false` value does not lower a score, reduce confidence, or refuse
    /// the corpus): making the signal's origin traceable and deciding how
    /// scoring should react to it are separate rounds of work.
    pub synthesis_focused: bool,
    /// Human-readable description of the corpus's intended use, copied
    /// from the corpus manifest.
    pub description: String,
    /// Version tag for how a "fragment" is defined/hashed in this corpus
    /// (`tools/build-fragment-corpus`'s `fragment_definition_version`) —
    /// distinct from `version` above, which identifies *this specific
    /// build*, not the definition it was built under.
    pub fragment_definition_version: String,
    /// Version tag for how the reference distribution (the percentile
    /// grid `FragmentCorpus::percentile_rank` queries) is computed.
    pub reference_distribution_version: String,
}

/// How well-precedented this molecule's fragments are relative to the
/// configured reference corpus (`AnalysisConfig.fragment_model`) — **an
/// explanatory reference-corpus signal, not a direct synthetic-difficulty
/// term.** `None` unless a corpus is configured, same as before round 21.
///
/// Round 17–19 wired this signal into `overall.difficulty` as a capped
/// correction term (option A). Round 20's cross-corpus robustness test
/// (`tasks/upstream_and_corpus_research.md` Part 7) found that unsafe: two
/// honestly-labeled, real, synthesis-focused reference corpora (ORD,
/// SynRXN) disagreed with each other on this signal's penalty/support
/// direction 34.6% of the time over 500 probe molecules — worse agreement
/// than either had with a bioactivity-focused corpus (ChEMBL) — and plain
/// pyridine (no plausible synthetic-difficulty story) swung between
/// `LikelyAccessible` and `HighlyChallenging` purely from which corpus was
/// configured, driven entirely by this signal's own uncapped penalty term.
/// "Rare in the configured reference corpus" and "difficult to synthesize"
/// are different claims — round 21 (option C) makes that distinction
/// structural rather than a caveat a reader has to remember: this type
/// exists specifically so `fragment_precedent` can never again silently
/// move `overall.difficulty`. See `rules.rs`'s "Fragment precedent"
/// section for the full history and reasoning.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FragmentPrecedentEvidence {
    /// `precedent_penalty - precedent_support`, in `-1.0..=1.0`. Negative =
    /// this molecule's fragments are strongly precedented (support side) in
    /// the configured corpus; positive = weakly precedented (penalty side).
    /// Purely descriptive — never added to or subtracted from
    /// `overall.difficulty`.
    pub signed_signal: f64,
    /// `max(signed_signal, 0.0)` — how weakly precedented this molecule's
    /// fragments are, uncapped.
    pub precedent_penalty: f64,
    /// `max(-signed_signal, 0.0)` — how strongly precedented this
    /// molecule's fragments are, uncapped. No support cap applies (round
    /// 21) — the cap existed only to bound this signal's effect on
    /// `overall.difficulty`, which it no longer has.
    pub precedent_support: f64,
    /// How reliable this specific signal is — flat `1.0` in v0.1, same
    /// caveat as before round 21: no sampling-uncertainty model yet for
    /// how corpus size/coverage should discount a percentile estimate.
    pub confidence: ProbabilityLikeScore,
    /// References into `SynthesizabilityReport.findings` for the
    /// `FragmentPrecedentWeak`/`FragmentPrecedentStrong` finding, if the
    /// signal cleared the display threshold — at most one entry.
    pub findings: Vec<FindingRef>,
}

/// The top-level report returned by `analyze`/`analyze_smiles`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SynthesizabilityReport {
    /// The bucketed verdict, difficulty/synthesizability scores, and
    /// overall confidence.
    pub overall: OverallAssessment,
    /// Each implemented difficulty-contributing component's own score, in
    /// isolation. `fragment_precedent` is **not** here — see
    /// [`SynthesizabilityReport::fragment_precedent`] below.
    pub components: ComponentScores,
    /// Every finding raised by every component, in a single flat list.
    pub findings: Vec<Finding>,
    /// Findings ranked by contribution magnitude, highest first — the
    /// "Dominant penalties" list in AGENTS.md §4.1/§15. Access via
    /// [`SynthesizabilityReport::dominant_penalties`]. Never contains a
    /// `fragment_precedent` entry (round 21) — this list means "actually
    /// moved `overall.difficulty`," and `fragment_precedent` no longer
    /// does.
    pub dominant_penalties: Vec<Contribution>,
    /// Factors that *reduce* difficulty, ranked by magnitude, highest
    /// first. Empty in v0.1 (round 21 removed `fragment_precedent`'s
    /// precedent-support case, its only source) — kept in the schema for
    /// forward compatibility with a future difficulty-reducing component,
    /// not fragment_precedent-specific machinery.
    pub dominant_supports: Vec<Contribution>,
    /// How well-precedented this molecule's fragments are relative to the
    /// configured reference corpus (`AnalysisConfig.fragment_model`) —
    /// explanatory evidence, **not** a contributor to `overall.difficulty`
    /// (round 21 / option C; see [`FragmentPrecedentEvidence`]'s own doc
    /// for the full reasoning). `None` unless a corpus is configured — no
    /// corpus ships with yomitoki itself (AGENTS.md §5.4).
    pub fragment_precedent: Option<FragmentPrecedentEvidence>,
    /// Derived from `findings` regardless of `overall.verdict` — a finding
    /// is real whether or not the molecule is also `OutOfDomain`/
    /// `Indeterminate` for an unrelated reason, so a suggestion can appear
    /// alongside either verdict. Only 3 of `SuggestionCode`'s 6 variants
    /// are reachable in v0.1; see `suggestions.rs`/`docs/architecture.md`.
    pub suggestions: Vec<SimplificationSuggestion>,
    /// Input-quality/domain-applicability signals, kept separate from
    /// `overall` (AGENTS.md §5.6).
    pub applicability: ApplicabilityReport,
    /// Schema/tool/ruleset versions this report was produced under.
    pub provenance: Provenance,
}

impl SynthesizabilityReport {
    /// Findings sorted by contribution magnitude, highest first — the
    /// "Dominant penalties" list in AGENTS.md §4.1/§15.
    pub fn dominant_penalties(&self) -> &[Contribution] {
        &self.dominant_penalties
    }

    /// Difficulty-*reducing* findings sorted by magnitude, highest first —
    /// the "Dominant supports" counterpart to
    /// [`SynthesizabilityReport::dominant_penalties`]. Empty unless a
    /// fragment corpus is configured and produced a precedent-support
    /// finding.
    pub fn dominant_supports(&self) -> &[Contribution] {
        &self.dominant_supports
    }
}
