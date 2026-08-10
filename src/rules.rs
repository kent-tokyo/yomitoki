//! Named thresholds and weights (AGENTS.md §20, §29: "重みやthresholdをコード内に
//! 散在させず、versioned rulesetに集約する" / "magic numberを散在させない").
//!
//! Every number a component or the aggregator uses to make a decision lives
//! here, named, with a one-line rationale. Bump [`RULESET_VERSION`] whenever
//! any constant below changes — it's recorded in every report's
//! `Provenance`.

/// Bumped whenever any constant in this module changes; recorded in every
/// report's [`crate::Provenance`] and, via the `#[doc(hidden)]` re-export
/// at the crate root, in `tools/build-fragment-corpus`'s manifest
/// provenance too.
pub const RULESET_VERSION: &str = "0.9.0";

// ---------------------------------------------------------------------------
// Applicability
// ---------------------------------------------------------------------------

/// Curated organic-chemistry element subset. AGENTS.md §28 explicitly rules
/// out full periodic-table coverage as a v0.1 goal; anything outside this
/// set is treated as out-of-domain rather than silently scored.
pub const SUPPORTED_ELEMENTS: &[chematic::core::Element] = &[
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

/// Confidence multiplier applied when stereo analysis could not be run at
/// all (currently: any negatively charged atom — see
/// `components::has_negatively_charged_atom`'s doc for why). Mutually
/// exclusive with [`CONFIDENCE_PENALTY_STEREO_INCOMPLETE`] per molecule
/// (either the real check ran, or it didn't), and deliberately a stronger
/// penalty than that one: "zero stereo information because the check
/// itself couldn't run" is a bigger gap than "checked, and some centers
/// happen to be unspecified." Not as severe as
/// [`CONFIDENCE_PENALTY_UNUSUAL_VALENCE`]'s territory either — a charged
/// atom isn't itself a structural irregularity, it's a tooling limitation
/// unrelated to whether the input molecule is well-formed.
pub(crate) const CONFIDENCE_PENALTY_STEREO_UNCHECKABLE: f64 = 0.6;

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
/// reachable from the `perception`/`chem` features yomitoki depends on.
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
/// names as a problem with existing tools. `fragment_rarity` (§5.4)
/// corrects this once a corpus is configured — round 16 found the
/// original formula did the *opposite* end-to-end; round 17's
/// corpus-relative redesign (see `rules::FRAGMENT_PRECEDENT_FINDING_
/// THRESHOLD`'s doc / the "Fragment rarity" section above) fixes the
/// documented case specifically: dodecane's `overall.difficulty` measured
/// `0.068` → `0.000` against the real 200k-molecule ChEMBL corpus
/// (`--fragment-corpus`), the corrected direction. Not a general
/// calibration claim — see that section's own caveat about corpus-domain
/// bias (ChEMBL reflects bioactivity relevance, not raw synthetic
/// accessibility; some genuinely-easy-but-bioactivity-atypical scaffolds
/// can still score *more* difficult with a corpus configured).
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
/// difficult to make. `fragment_rarity` (§5.4) corrects for this the same
/// way it corrects the rotatable-bond term (`SIZE_WEIGHT_PER_ROTATABLE_
/// BOND`) once a corpus is configured — round 17's corpus-relative
/// redesign measured aspirin's `overall.difficulty` at `0.273` → `0.095`
/// against the real 200k-molecule ChEMBL corpus (the corrected direction;
/// round 16 found the original formula went the other way). Paracetamol
/// similarly measured `0.243` → `0.095`. See this file's "Fragment
/// rarity" section for the formula and its corpus-domain-bias caveat.
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

/// Burden per distinct functional-group cluster (`chematic::chem::
/// identify_functional_groups`, Ertl 2017) *beyond the first*. AGENTS.md
/// §5.5's "dense functionalization" example — a separate signal from Brenk
/// alerts above: this counts ordinary, non-reactive functional groups too,
/// on the theory that more independent reactive/functional regions in one
/// molecule means more competing sites to sequence and protect, regardless
/// of whether any single one is individually unstable.
///
/// The first cluster is free (weight applies to `count.saturating_sub(1)`):
/// a single connected region of functionality is the ordinary case for any
/// non-trivial organic molecule (confirmed empirically — ethanol, a bare
/// C-O environment, is `count == 1`) and isn't itself evidence of anything
/// unusual; burden only starts once a *second*, topologically disconnected
/// region exists.
///
/// Known weak spot (documented, not hidden — same treatment as
/// [`SIZE_WEIGHT_PER_ROTATABLE_BOND`]/[`FG_WEIGHT_PER_REACTIVE_GROUP`]'s
/// caveats): `identify_functional_groups` merges adjacent/fused heteroatom
/// environments into one cluster, so a single densely interconnected
/// polyfunctional system undercounts here — confirmed empirically:
/// glucose (6 hydroxyls in one ring) and penicillin V (β-lactam +
/// thioether + amide + carboxylic acid + aryl ether, all ring-fused) both
/// come back as `count == 1`, identical to ethanol. This term only catches
/// *disconnected* multi-site burden (e.g. several separate esters on one
/// scaffold), not fused polyfunctional density.
pub(crate) const FG_WEIGHT_PER_DISTINCT_GROUP: f64 = 0.08;

/// Distinct functional-group cluster count above which a
/// `FunctionalGroupDense` finding is emitted. Chosen empirically: real
/// drug-like molecules with ordinary functionality (aspirin, paracetamol)
/// both come back at `count == 2`; a threshold of 3 (finding fires at 4+)
/// sits comfortably above that baseline while still catching genuinely
/// multi-site molecules (a tetraester on a branched core comes back at
/// `count == 4`; five scattered, non-adjacent amines come back at
/// `count == 5`).
pub(crate) const FG_DENSE_GROUP_COUNT_THRESHOLD: usize = 3;

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

// ---------------------------------------------------------------------------
// Fragment rarity (AGENTS.md §5.4) — corpus-relative signed precedent
// ---------------------------------------------------------------------------
//
// Round 16 found the original formula (`raw = WEIGHT * (1.0 -
// mean_document_frequency)`, added unconditionally to difficulty) broken,
// not merely untuned: real molecules' mean document frequency in a diverse
// corpus rarely exceeds ~0.3–0.4 even for ordinary fragments, so that
// formula added positive burden for essentially every molecule, common or
// not — confirmed end-to-end (aspirin `0.273 → 0.428`, dodecane
// `0.068 → 0.227`, both *worse* once a corpus was configured, for exactly
// the cases it was meant to correct).
//
// Round 17's redesign, per real round-16 measurement of the 200k-molecule
// corpus's own molecule-level mean-document-frequency distribution
// (`tools/build-fragment-corpus`'s `reference_distribution`, quantiles:
// q01=0.079 q05=0.108 q10=0.123 q25=0.142 q50=0.161 q75=0.182 q90=0.204
// q95=0.218 q99=0.250 — a fairly narrow, roughly symmetric distribution
// around the median, no obvious plateau suggesting a hard neutral
// dead-band boundary, which is why none is used below):
//
//   p = corpus.percentile_rank(mean_document_frequency)  // empirical CDF
//   signed_signal = 1.0 - 2.0 * p                         // in [-1, 1]
//   rarity_penalty    = max(signed_signal, 0.0)           // p < 0.5: rare
//   precedent_support = max(-signed_signal, 0.0)          // p > 0.5: common
//
// `signed_signal` is continuous and crosses zero exactly at the corpus
// median (p = 0.5), so it already provides a *soft* neutral zone around
// "typical for this corpus" without needing a separate hard dead-band on
// top — a second free parameter that round-16's single-corpus data
// wouldn't justify picking a width for anyway.
//
// `precedent_support` is capped in `analyze::analyze` (not here — the cap
// needs `size_topology`/`functional_group_liability`'s own contributions,
// only known at aggregation time) at those two components' combined
// contribution: strong fragment precedent can offset the "this looks like
// an unusual/large substituent pattern" burden those two componen ts
// capture, but must never zero out `ring_topology`/`stereochemical_burden`
// burden just because a molecule's fragments are individually common — a
// bridged cage or a stereocenter-dense core is exactly as hard to build
// regardless of how precedented its individual fragments are.
//
// Deliberately no saturating burden transform (unlike every other
// component, AGENTS.md §5.1): `signed_signal` is already bounded to
// `[-1, 1]` by construction (a percentile can't exceed `[0, 1]`), so
// there's no unbounded `raw` value to saturate — applying `1 - exp(-raw/
// scale)` on top would only compress an already-bounded signal further,
// for no expressed benefit.
//
// Known caveat, reported honestly rather than tuned away: corpus-domain
// bias. Round 17's validation panel, run end-to-end against the real
// 200k-molecule ChEMBL-37 corpus, confirmed the three documented target
// cases fixed in the intended direction —
//   aspirin              0.273 -> 0.095  (was worse in round 16, now better)
//   paracetamol          0.243 -> 0.095
//   dodecane             0.068 -> 0.000
// — but also surfaced four structurally-legitimate molecules that got
// substantially *harder*, not easier, once the corpus was configured:
//   caffeine              0.287 -> 0.516  (MODERATELY_ACCESSIBLE -> CHALLENGING)
//   norbornane (bridged)  0.341 -> 0.985  (MODERATELY_ACCESSIBLE -> HIGHLY_CHALLENGING)
//   spiro-decane           0.199 -> 1.000  (LIKELY_ACCESSIBLE -> HIGHLY_CHALLENGING)
//   stereocenter-dense     0.275 -> 1.000  (MODERATELY_ACCESSIBLE -> HIGHLY_CHALLENGING)
// This was checked and is not a formula or cap bug: independently
// recomputing each molecule's raw mean document frequency and percentile
// against the corpus's own `fragment_frequencies.json`/
// `reference_distribution` reproduces the same numbers the formula uses
// (e.g. caffeine mean_df=0.153 -> p=0.386; norbornane mean_df=0.135 ->
// p=0.178). The formula and support cap are behaving exactly as
// specified — the corpus itself is the source of the surprise: ChEMBL is
// a bioactivity-screening corpus, not a synthetic-accessibility corpus,
// so its fragment-frequency table reflects which substructures show up in
// bioassay-tested compounds, not which substructures are common building
// blocks in synthesis. A caffeine-like fused purine core or a bridged
// bicyclic ring can be genuinely easy to source/build while still being
// under-represented in ChEMBL's compound population, so it registers as
// "rare" here even though "rare in ChEMBL" and "hard to synthesize" are
// not the same claim. `fragment_rarity`'s corpus-relative percentile is
// only ever as good as the corpus it's given — swapping in a
// synthesis-focused corpus (e.g. a reaction-precursor or building-block
// database) instead of ChEMBL would be the natural next step to reduce
// this bias, not a further formula change.

/// Minimum `|signed_signal|` for `fragment_rarity` to emit a
/// `Finding`/`Contribution` at all — purely a *display* threshold ("is
/// this worth surfacing as evidence"), not a scoring dead-band:
/// `signed_signal` still applies to `overall.difficulty` continuously
/// regardless of this constant. `0.1` (roughly `p` outside `[0.45,
/// 0.55]`) is a first-pass round number for keeping near-median molecules
/// finding-free rather than a tuned value — revisit with more real-corpus
/// validation data than round 16/17's if it turns out too
/// noisy/insensitive in practice.
pub(crate) const FRAGMENT_PRECEDENT_FINDING_THRESHOLD: f64 = 0.1;

/// Below this confidence (and absent a hard applicability failure), the
/// verdict is `Indeterminate` rather than a difficulty-based bucket.
/// Strictness-dependent — see [`indeterminate_confidence_threshold`].
/// `Standard`'s value (0.45) is deliberately above the lowest confidence
/// floor applicability's soft penalties can combine to reach at Standard
/// strictness — a threshold below that floor would make `Indeterminate`
/// unreachable at that strictness level. Applicability currently has three
/// soft-penalty sources (`components/applicability.rs`); two are mutually
/// exclusive per molecule (`CONFIDENCE_PENALTY_STEREO_INCOMPLETE` and
/// `CONFIDENCE_PENALTY_STEREO_UNCHECKABLE` can't both apply — either the
/// stereo check ran or it didn't), so the actual floor is
/// `CONFIDENCE_PENALTY_UNUSUAL_VALENCE * CONFIDENCE_PENALTY_STEREO_
/// UNCHECKABLE = 0.5 * 0.6 = 0.3` (lower than the pre-`STEREO_UNCHECKABLE`
/// floor of 0.5 * 0.85 = 0.425), well below 0.45.
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

// ---------------------------------------------------------------------------
// Simplification suggestions
// ---------------------------------------------------------------------------

/// Confidence assigned to every v0.1 simplification suggestion (AGENTS.md
/// §9), regardless of which finding it was derived from. Deliberately flat
/// rather than per-suggestion-code: nothing in v0.1 has been calibrated
/// against real synthesis outcomes (no corpus exists — same gap as
/// `fragment_rarity`), so differentiating confidence between, say, a
/// bridged-ring suggestion and a stereocenter-density one would imply a
/// precision this crate doesn't have. `0.5` reads as "this follows from our
/// own scoring model's causality, not from validated outcomes" — high
/// enough to be worth surfacing, not so high it reads as calibrated.
pub(crate) const SUGGESTION_CONFIDENCE_HEURISTIC: f64 = 0.5;
