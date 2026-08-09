//! Named thresholds and weights (AGENTS.md §20, §29: "重みやthresholdをコード内に
//! 散在させず、versioned rulesetに集約する" / "magic numberを散在させない").
//!
//! Every number a component or the aggregator uses to make a decision lives
//! here, named, with a one-line rationale. Bump [`RULESET_VERSION`] whenever
//! any constant below changes — it's recorded in every report's
//! `Provenance`.

pub const RULESET_VERSION: &str = "0.4.0";

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
// Size / topology burden
// ---------------------------------------------------------------------------

/// Burden per dalton of molecular weight. Weight, not heavy-atom count, is
/// used as the size signal — the two are highly correlated, and MW carries
/// a more recognizable reference point (see
/// [`SIZE_LARGE_MOLECULAR_WEIGHT_THRESHOLD`]) than an arbitrary atom-count
/// cutoff would.
pub(crate) const SIZE_WEIGHT_PER_MOLECULAR_WEIGHT_UNIT: f64 = 0.0006;

/// Burden per acyclic rotatable bond. AGENTS.md §5.1 lists rotatable bond
/// count as a candidate structural-burden indicator directly; the
/// rationale used here is "more independent single-bond connections
/// roughly tracks more bond-forming steps," not a flexibility/ADME claim
/// (Veber's rule, which uses the same threshold, is about oral
/// bioavailability, a different question — the number is reused, the
/// rationale is not).
///
/// Known weak spot (documented, not hidden): this over-penalizes simple,
/// commercially available long unbranched chains, which have many
/// rotatable bonds but essentially no synthetic difficulty — exactly the
/// "structural complexity vs. actual difficulty" conflation AGENTS.md §2
/// names as a problem with existing tools. Fragment rarity (§5.4, not yet
/// implemented) is what's supposed to correct for this by recognizing such
/// fragments as common/precedented; until it exists, this component alone
/// will overstate difficulty for that specific case.
pub(crate) const SIZE_WEIGHT_PER_ROTATABLE_BOND: f64 = 0.03;

/// Scale in the `normalized = 1 - exp(-raw / scale)` burden transform
/// (AGENTS.md §5.1: burden should be non-linear).
pub(crate) const SIZE_BURDEN_SCALE: f64 = 2.0;

/// Molecular weight (daltons) above which a `SizeLargeMolecularWeight`
/// finding is emitted. 500 Da is a widely recognized "large molecule"
/// reference point (the same number appears in Lipinski's Rule of Five),
/// reused here purely as a size cutoff — not a druglikeness/permeability
/// claim, which is what that rule is actually about.
pub(crate) const SIZE_LARGE_MOLECULAR_WEIGHT_THRESHOLD: f64 = 500.0;

/// Rotatable bond count above which a `SizeHighRotatableBondCount` finding
/// is emitted. Matches the same number used in Veber's rule; see the
/// rationale note on [`SIZE_WEIGHT_PER_ROTATABLE_BOND`] for why the
/// citation is reused but the underlying claim is not.
pub(crate) const SIZE_HIGH_ROTATABLE_BOND_THRESHOLD: usize = 10;

// ---------------------------------------------------------------------------
// Stereochemical burden
// ---------------------------------------------------------------------------

/// Burden per tetrahedral stereocenter (specified or unspecified — the
/// molecule needs the same synthetic control over its configuration either
/// way; whether the input SMILES wrote it out is an input-quality/
/// confidence concern, handled separately by the applicability component,
/// not a difficulty concern).
pub(crate) const STEREO_WEIGHT_PER_CENTER: f64 = 0.12;

/// Multiplier on stereocenter density (`total_centers / heavy_atom_count`)
/// — AGENTS.md §8.3's own worked example frames the concern as "three
/// defined stereocenters within a compact eight-heavy-atom region," i.e.
/// concentration, not raw count alone: the same center count in a much
/// larger molecule leaves more room for orthogonal, staged control.
pub(crate) const STEREO_WEIGHT_DENSITY: f64 = 0.6;

/// Scale in the `normalized = 1 - exp(-raw / scale)` burden transform
/// (AGENTS.md §5.1: burden should be non-linear).
pub(crate) const STEREO_BURDEN_SCALE: f64 = 1.5;

/// Stereocenter density above which a `StereoDensityHigh` finding is
/// emitted. Chosen independently (not copied from AGENTS.md §8.2's
/// worked-example value of 0.12, which pairs with an unspecified,
/// differently-defined local density metric, not this whole-molecule
/// `centers / heavy_atom_count` ratio — reusing that number without
/// knowing what it actually measures would be false precision, not
/// consistency). 0.25 means roughly one in every four heavy atoms is a
/// stereocenter.
pub(crate) const STEREO_DENSITY_FINDING_THRESHOLD: f64 = 0.25;

// ---------------------------------------------------------------------------
// Functional-group liabilities
// ---------------------------------------------------------------------------

/// Burden per distinct triggered Brenk (2008) structural alert
/// (`chematic::chem::brenk_matches_detailed`). AGENTS.md §5.5's "reactive/
/// unstable functional groups" category, deliberately scoped to Brenk's own
/// set rather than a hand-curated one — reusing an existing, published set
/// is narrower than writing new SMARTS from scratch. Brenk also already
/// includes `strained_ring_three`/`strained_ring_four`, covering AGENTS.md
/// §5.5's "strained motifs" example for free. Dense functionalization,
/// chemoselectivity burden, and oxidation-state combinations are NOT
/// covered — chematic has no oxidation-state API (confirmed absent) and no
/// FG-density metric wired in yet; see `docs/architecture.md`.
///
/// Known weak spot (documented, not hidden — same treatment as
/// [`SIZE_WEIGHT_PER_ROTATABLE_BOND`]'s caveat): Brenk et al. 2008 was
/// validated as a med-chem screening-library *desirability* filter
/// (reactivity toward assay components, metabolic liability, promiscuity),
/// not a synthetic-difficulty signal. Several of its ~105 alerts fire on
/// common, cheaply-precedented groups — confirmed by probing real drugs:
/// aspirin (`CC(=O)Oc1ccccc1C(=O)O`) trips `phenol`, `phenolic_aldehyde`,
/// `active_ester`, and `acetal_ketal`; paracetamol trips `phenol`,
/// `aniline`, and `secondary_amine`. Neither molecule is remotely
/// difficult to make. Fragment rarity (§5.4, not yet implemented) is
/// expected to correct for this the same way it's expected to correct the
/// rotatable-bond term, by recognizing such fragments as common/
/// precedented.
pub(crate) const FG_WEIGHT_PER_REACTIVE_GROUP: f64 = 0.12;

/// Scale in the `normalized = 1 - exp(-raw / scale)` burden transform
/// (AGENTS.md §5.1: burden should be non-linear).
pub(crate) const FG_BURDEN_SCALE: f64 = 1.5;

/// Per-finding confidence when `brenk_matches_detailed` reports an alert
/// whose VF2 enumeration was cut off by the visit budget before completing
/// (empty `atom_indices` — see that function's doc: still a real flagged
/// alert, just one whose full match extent couldn't be resolved). Lower
/// than the `1.0` used for a fully-resolved match, never dropped silently
/// (AGENTS.md §4.4: abstain/flag uncertainty, don't hide it).
pub(crate) const FG_CONFIDENCE_BUDGET_EXHAUSTED: f64 = 0.5;

// ---------------------------------------------------------------------------
// Aggregation / verdict
// ---------------------------------------------------------------------------

/// Weighted **sum**, not a weighted average — matches AGENTS.md §20's own
/// formula (`difficulty = w_topology*topology + w_rings*ring_topology +
/// ...`), which adds each `w * normalized` term directly rather than
/// dividing by the weight total. `ProbabilityLikeScore::new` clamps the
/// result to `0.0..=1.0`, so weights don't need to sum to 1.
///
/// Ring topology's weight is 1.0 (full pass-through): a molecule with only
/// ring-topology burden and negligible size burden scores identically to
/// the single-component model this crate started with — adding a second
/// component should never silently water down a signal the first
/// component already gave full confidence in. Size topology is additive
/// on top, at a fraction of that weight, so it registers as extra burden
/// when large but can't drag a small, ring-driven molecule down into a
/// lower verdict bucket by "diluting" the average.
pub(crate) const AGGREGATE_WEIGHT_RING_TOPOLOGY: f64 = 1.0;
pub(crate) const AGGREGATE_WEIGHT_SIZE_TOPOLOGY: f64 = 0.4;
pub(crate) const AGGREGATE_WEIGHT_STEREOCHEMICAL_BURDEN: f64 = 0.5;
pub(crate) const AGGREGATE_WEIGHT_FUNCTIONAL_GROUP_LIABILITY: f64 = 0.4;

/// Below this confidence (and absent a hard applicability failure), the
/// verdict is `Indeterminate` rather than a difficulty-based bucket.
/// Strictness-dependent — see [`indeterminate_confidence_threshold`].
/// `Standard`'s value (0.45) is deliberately above the confidence floor
/// applicability's two soft penalties combined can reach (0.5 * 0.85 =
/// 0.425, see `components/applicability.rs`) — a threshold below that
/// floor would make `Indeterminate` unreachable at that strictness level.
const INDETERMINATE_CONFIDENCE_THRESHOLD_LENIENT: f64 = 0.3;
const INDETERMINATE_CONFIDENCE_THRESHOLD_STANDARD: f64 = 0.45;
const INDETERMINATE_CONFIDENCE_THRESHOLD_STRICT: f64 = 0.6;

/// The confidence floor below which the verdict becomes `Indeterminate`,
/// for a given [`crate::config::Strictness`]. Higher strictness abstains
/// more readily (higher threshold).
pub(crate) fn indeterminate_confidence_threshold(strictness: crate::config::Strictness) -> f64 {
    use crate::config::Strictness;
    match strictness {
        Strictness::Lenient => INDETERMINATE_CONFIDENCE_THRESHOLD_LENIENT,
        Strictness::Standard => INDETERMINATE_CONFIDENCE_THRESHOLD_STANDARD,
        Strictness::Strict => INDETERMINATE_CONFIDENCE_THRESHOLD_STRICT,
    }
}

/// Difficulty upper bounds for the three lowest verdict buckets; anything
/// above the last bound is `HighlyChallenging`.
pub(crate) const DIFFICULTY_LIKELY_ACCESSIBLE_MAX: f64 = 0.25;
pub(crate) const DIFFICULTY_MODERATE_MAX: f64 = 0.5;
pub(crate) const DIFFICULTY_CHALLENGING_MAX: f64 = 0.75;
