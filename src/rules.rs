//! Named thresholds and weights (AGENTS.md §20, §29: "重みやthresholdをコード内に
//! 散在させず、versioned rulesetに集約する" / "magic numberを散在させない").
//!
//! Every number a component or the aggregator uses to make a decision lives
//! here, named, with a one-line rationale. Bump [`RULESET_VERSION`] whenever
//! any constant below changes — it's recorded in every report's
//! `Provenance`.

pub const RULESET_VERSION: &str = "0.8.0";

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
/// names as a problem with existing tools. `fragment_rarity` (§5.4) was
/// supposed to correct this by recognizing such fragments as
/// common/precedented, and is now implemented — but round 16 found it
/// currently does the *opposite* when tested end-to-end against a real
/// corpus (dodecane's `overall.difficulty` went from `0.068` to `0.227`
/// once a corpus was configured, not down). See `FRAGMENT_RARITY_WEIGHT`'s
/// doc for the root cause (a formula defect, not a tuning one) — until
/// it's redesigned, this component alone still overstates difficulty for
/// this case, and configuring a corpus makes it worse, not better.
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
/// difficult to make. `fragment_rarity` (§5.4) was expected to correct for
/// this the same way it was expected to correct the rotatable-bond term
/// (`SIZE_WEIGHT_PER_ROTATABLE_BOND`) by recognizing such fragments as
/// common/precedented — round 16 found it currently does the opposite for
/// aspirin end-to-end (`overall.difficulty` `0.273` → `0.428` once a
/// corpus is configured). See `FRAGMENT_RARITY_WEIGHT`'s doc for why (a
/// formula defect, not a tuning one).
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
/// Only applied when a fragment corpus is configured
/// (`AnalysisConfig.fragment_model`) — `fragment_rarity` is `None`
/// otherwise and contributes nothing. `0.4` is a first-pass value matching
/// `size_topology`/`functional_group_liability`'s weight class ("medium
/// importance"), not a calibrated one — no real-molecule validation data
/// exists yet to tune it against (same gap the other four weights have; see
/// `docs/architecture.md`'s Non-goals). Does not itself rebalance the other
/// four weights; whether aspirin/long-chain-alkane's known over
/// -penalization (see `RING_WEIGHT_*`'s and `SIZE_WEIGHT_PER_ROTATABLE_
/// BOND`'s doc comments) nets out correctly once this term is added is an
/// empirical question for real molecules, not something this constant's
/// value alone decides.
pub(crate) const AGGREGATE_WEIGHT_FRAGMENT_RARITY: f64 = 0.4;

// ---------------------------------------------------------------------------
// Fragment rarity (AGENTS.md §5.4)
// ---------------------------------------------------------------------------

/// A fragment's document frequency (occurrence count / corpus size) below
/// this counts as "rare" for the `rare fragment count` figure AGENTS.md
/// §5.4 requires the component to report. Purely descriptive — it does not
/// drive `raw`/`normalized` (see `FRAGMENT_RARITY_WEIGHT`'s doc for why
/// mean document frequency, not a threshold count, is what's used there).
/// `0.05`: a first-pass round number, not tuned — see
/// `tasks/upstream_and_corpus_research.md` (gitignored) Part 5 for the
/// real-corpus numbers this was picked in view of (aspirin/dodecane mean
/// document frequency ~0.24–0.27 against a real 200k-molecule ChEMBL
/// corpus; a structurally atypical control's mean was ~0.033).
pub(crate) const FRAGMENT_RARITY_DF_THRESHOLD: f64 = 0.05;

/// `raw = FRAGMENT_RARITY_WEIGHT * (1.0 - mean_document_frequency)` across
/// a molecule's fragments. Mean, not minimum: round-14 corpus testing
/// (`tasks/upstream_and_corpus_research.md` Part 5) found minimum document
/// frequency alone doesn't separate known-common molecules from a
/// known-atypical control — a common molecule can still contain one
/// specific, narrowly-precedented fragment (aspirin's rarest measured
/// fragment was seen in under 0.04% of a 200k-molecule corpus) without the
/// molecule as a whole being unusual. Mean document frequency across *all*
/// of a molecule's fragments was the statistic that actually separated the
/// tested cases.
///
/// **Round 16, run end-to-end against a real 200k-molecule ChEMBL corpus:
/// this formula does not correct the false positives it was built to
/// correct — it makes both of them worse.** Aspirin's `overall.difficulty`
/// went from `0.273` (no corpus) to `0.428` (corpus configured); dodecane's
/// went from `0.068` to `0.227`. Round 14 confirmed *relative* ranking is
/// right (rare-control > aspirin/dodecane, e.g. a perfluorooctyl chain's
/// `fragment_rarity.normalized` of `0.475` versus aspirin's `0.386`), but
/// there is no baseline "common enough, contribute ~0" case: real
/// molecules' mean document frequency in a diverse corpus rarely exceeds
/// ~0.3–0.4 even for genuinely ordinary fragments (round-14 data: known
/// -common cases ~0.24–0.27), so `1.0 - mean_document_frequency` sits at
/// `~0.7` for essentially every real molecule, common or not, and always
/// adds positive burden. This is a formula defect, not a constant-tuning
/// one — no value of `FRAGMENT_RARITY_WEIGHT` or `FRAGMENT_RARITY_BURDEN_
/// SCALE` fixes it, since the problem is what `raw` is computed *from*, not
/// how it's scaled afterward. A fix needs a formula with an actual
/// "common enough → ~zero" reference point (e.g. relative to the corpus's
/// own mean-document-frequency distribution, not an absolute `1.0`
/// ceiling that real corpora never approach) — not attempted here; this is
/// a real design decision, not a value to guess at. Until redesigned,
/// don't present `fragment_rarity` as correcting the rotatable-bond/Brenk
/// -alert over-penalization cases it was meant to (see `SIZE_WEIGHT_PER_
/// ROTATABLE_BOND`'s and `FG_WEIGHT_PER_REACTIVE_GROUP`'s doc comments,
/// and the READMEs' Limitations sections) — it currently does the
/// opposite for both.
pub(crate) const FRAGMENT_RARITY_WEIGHT: f64 = 1.0;

/// Non-linear burden scale (AGENTS.md §5.1), same saturating-transform
/// convention as every other component.
///
/// Known weak spot (documented, not hidden — same practice as `SIZE_WEIGHT_
/// PER_ROTATABLE_BOND`'s): with `FRAGMENT_RARITY_WEIGHT = 1.0`, `raw` is
/// bounded to `0.0..=1.0` (since `1.0 - mean_document_frequency` can't
/// exceed 1.0), which caps `normalized` at `1.0 - exp(-1.0/1.5) ≈ 0.487` —
/// even a molecule sharing *zero* fragments with the corpus never reaches
/// the "severe burden" range other components can reach (e.g. a
/// heavily-fused ring system's `ring_topology` score). Left as-is rather
/// than re-tuned blindly: real molecules' mean document frequency rarely
/// approaches either extreme (round-14 data: known-common cases ~0.24–0.27,
/// an atypical control ~0.033 — none near 0.0 or 1.0), so which of
/// `FRAGMENT_RARITY_WEIGHT`/this scale should move, and by how much, needs
/// more real-corpus data than three data points provide, not a guess.
pub(crate) const FRAGMENT_RARITY_BURDEN_SCALE: f64 = 1.5;

/// How many of a molecule's rarest fragments to name in a
/// `FindingCode::FragmentRarityHigh` finding's explanation text. Fragment
/// hashes aren't chemically interpretable on their own (no reverse mapping
/// from hash to substructure exists in chematic's API) — this exists so
/// the explanation is concrete evidence, not just a count.
pub(crate) const FRAGMENT_RARITY_REPORT_COUNT: usize = 3;

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
