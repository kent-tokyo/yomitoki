//! Named thresholds and weights (AGENTS.md §20, §29: "重みやthresholdをコード内に
//! 散在させず、versioned rulesetに集約する" / "magic numberを散在させない").
//!
//! Every number a component or the aggregator uses to make a decision lives
//! here, named, with a one-line rationale. Bump [`RULESET_VERSION`] whenever
//! any constant below changes — it's recorded in every report's
//! `Provenance`.

pub const RULESET_VERSION: &str = "0.1.0";

// ---------------------------------------------------------------------------
// Applicability
// ---------------------------------------------------------------------------

/// Curated organic-chemistry element subset. AGENTS.md §28 explicitly rules
/// out full periodic-table coverage as a v0.1 goal; anything outside this
/// set is treated as out-of-domain rather than silently scored.
pub(crate) const SUPPORTED_ELEMENTS: &[chematic::core::Element] = &[
    chematic::core::Element::H,
    chematic::core::Element::B,
    chematic::core::Element::C,
    chematic::core::Element::N,
    chematic::core::Element::O,
    chematic::core::Element::F,
    chematic::core::Element::SI,
    chematic::core::Element::P,
    chematic::core::Element::S,
    chematic::core::Element::CL,
    chematic::core::Element::BR,
    chematic::core::Element::I,
];

/// Confidence multiplier applied when `validate_valence` reports any
/// violation. A soft penalty, not a hard `OutOfDomain` trigger — valence
/// heuristics can flag legitimate-but-unusual structures.
pub(crate) const CONFIDENCE_PENALTY_UNUSUAL_VALENCE: f64 = 0.5;

/// Confidence multiplier applied when at least one stereocenter is
/// unspecified. Soft penalty — incomplete stereo is common in real input
/// and not itself evidence the molecule is out of domain.
pub(crate) const CONFIDENCE_PENALTY_STEREO_INCOMPLETE: f64 = 0.85;

// ---------------------------------------------------------------------------
// Ring topology burden
// ---------------------------------------------------------------------------

/// Per-family weight for an isolated (non-fused) ring.
pub(crate) const RING_WEIGHT_SIMPLE: f64 = 0.15;
/// Base weight for a fused ring system, before the density term.
pub(crate) const RING_WEIGHT_FUSED_BASE: f64 = 0.35;
/// Multiplier on fusion density (shared-atom overlap fraction) for fused
/// systems — denser fusion (more shared atoms per ring) burdens more.
pub(crate) const RING_WEIGHT_FUSED_DENSITY: f64 = 0.5;
pub(crate) const RING_WEIGHT_SPIRO: f64 = 0.3;
pub(crate) const RING_WEIGHT_BRIDGED: f64 = 0.6;
/// Added on top of the kind-based weight when any ring in the family is at
/// least [`MACROCYCLE_MIN_RING_SIZE`] atoms.
pub(crate) const RING_WEIGHT_MACROCYCLE_BONUS: f64 = 0.25;

/// Ring size at/above which a ring counts as a macrocycle. Chosen to match
/// the threshold `chematic-3d`'s internal (non-public) macrocycle
/// classifier uses, for consistency even though that constant isn't
/// reachable from the `perception`/`chem` features rensei depends on.
pub(crate) const MACROCYCLE_MIN_RING_SIZE: usize = 9;

/// Fusion density above which a `RingFusedDense` finding is emitted (in
/// addition to the family already contributing fused burden).
pub(crate) const RING_FUSED_DENSITY_FINDING_THRESHOLD: f64 = 0.5;

/// Scale in the `normalized = 1 - exp(-raw / scale)` burden transform
/// (AGENTS.md §5.1: burden should be non-linear, not a simple capped sum).
pub(crate) const RING_BURDEN_SCALE: f64 = 1.5;

// ---------------------------------------------------------------------------
// Aggregation / verdict
// ---------------------------------------------------------------------------

/// Below this confidence (and absent a hard applicability failure), the
/// verdict is `Indeterminate` rather than a difficulty-based bucket.
pub(crate) const INDETERMINATE_CONFIDENCE_THRESHOLD: f64 = 0.4;

/// Difficulty upper bounds for the three lowest verdict buckets; anything
/// above the last bound is `HighlyChallenging`.
pub(crate) const DIFFICULTY_LIKELY_ACCESSIBLE_MAX: f64 = 0.25;
pub(crate) const DIFFICULTY_MODERATE_MAX: f64 = 0.5;
pub(crate) const DIFFICULTY_CHALLENGING_MAX: f64 = 0.75;
