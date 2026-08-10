# yomitoki architecture (v0.1)

This document defines the crate boundary, public API, report schema, component
interface, scoring direction, confidence/abstention contract, versioning
scheme, and non-goals for yomitoki v0.1. It reflects what is actually
implemented today, not the eventual full scope — see "Non-goals / deferred"
at the end for what's intentionally missing.

yomitoki reads and explains molecular structure — it does not modify,
optimize, or regenerate molecules. That functionality, if it's ever built,
belongs to a different, unrelated project.

## Crate boundary

Single crate, `yomitoki`, no workspace split. `fragment_rarity` now exists
and needs a `FragmentCorpus`, but that corpus is loaded at runtime from an
external directory (`FragmentCorpus::load_dir`, built with
`tools/build-fragment-corpus`) — no corpus is embedded in this crate, so
there's still no large embedded model to justify splitting into
`yomitoki-core`/`yomitoki-models`/`yomitoki-data`. That split is revisited
if/when a corpus ships *with* yomitoki by default, not merely because the
component that consumes one exists.

yomitoki depends on `chematic` (registry dependency, not a path dependency) for
all molecule representation, SMILES parsing, ring perception, and
stereochemistry. yomitoki does not reimplement any of that. See "chematic API
surface used" below for exactly what's called.

yomitoki does not depend on RENKIN, and RENKIN must never depend on yomitoki.
yomitoki never runs retrosynthesis search or template application.

## Public API

```rust
pub fn analyze(
    molecule: &chematic::core::Molecule,
    config: &AnalysisConfig,
) -> Result<SynthesizabilityReport, YomitokiError>;

pub fn analyze_smiles(
    smiles: &str,
    config: &AnalysisConfig,
) -> Result<SynthesizabilityReport, YomitokiError>;

pub fn analyze_batch(
    molecules: &[chematic::core::Molecule],
    config: &AnalysisConfig,
) -> Vec<Result<SynthesizabilityReport, YomitokiError>>;
```

`analyze_smiles` is `chematic::smiles::parse` followed by `analyze`. Parsing
is the only fallible step in the whole pipeline — a molecule that parses
successfully always returns `Ok(report)`, never `Err`, no matter how
difficult or out-of-domain it is. A hard-to-synthesize molecule is not an
error.

`analyze_batch` (AGENTS.md §18) is `molecules.iter().map(|m| analyze(m,
config)).collect()` — `result[i]` corresponds to `molecules[i]`, and one
molecule's result never depends on another's, so it's safe to parallelize
(e.g. with `rayon`'s `par_iter`) without changing output. Sequential in
v0.1: §18 only asks that parallelism be *possible* behind a feature flag
("Rayon利用はfeature flagでもよい"), not that it ship now, and nothing in
this crate's own use so far has shown a need for it. The CLI's own batch
mode (`run_batch` in `src/bin/yomitoki.rs`) does not call `analyze_batch` —
it reads records lazily from `SdfReader`/`SmilesRecordReader` and needs to
interleave per-record *parse* failures with analysis results in one pass,
a shape `analyze_batch`'s `&[Molecule]` signature (already-parsed input)
doesn't fit without an extra clone-and-reindex step for no behavioral
benefit; kept as two separate, both-correct code paths rather than forcing
one into the other's shape.

### CLI (`src/bin/yomitoki.rs`)

A thin binary over the same `analyze`/`analyze_smiles` entry points — no
scoring logic lives in the binary. Two modes:

* Single molecule: `yomitoki analyze "<SMILES>" [--format human|json|jsonl]`.
* Batch: `yomitoki analyze --input <file> [--format human|json|jsonl] [--output <file>]`,
  reading either a `.sdf` file (via `chematic::mol::SdfReader`) or a
  SMILES-per-line file (via `chematic::mol::SmilesRecordReader`).

Batch mode reads all records into a `Vec<(label, Result<Molecule, String>)>`
before analyzing — this is a deliberate choice, not an oversight: both
reader iterators surface a per-record `Result`, and mapping each item to a
labeled tuple before `.collect()`-ing into a plain `Vec` means one bad
record never short-circuits the collection (unlike collecting directly into
`Result<Vec<_>, _>`, which chematic's own `parse_sdf` convenience wrapper
does — yomitoki intentionally uses the lower-level `SdfReader` iterator
instead of that wrapper for this reason). A failed record becomes an error
entry in the output, at its original position, never a silent skip.
`SmilesRecordReader` has its own independent stop-on-error mechanism (its
`strict_parsing` option, which the CLI leaves at its default, `false` — the
"continue past a bad row" behavior, not the "stop" one) that is unrelated to
the `.collect()` short-circuiting above but produces the same required
outcome.

`jsonl` output uses one wrapper shape, `{"input", "report"|"error"}`, in
both single-molecule and batch mode — this was originally two different
shapes (a bare `SynthesizabilityReport` in single mode) until a review
caught that a downstream line-by-line parser would see incompatible schemas
depending on which invocation form produced the output; unified before this
was ever released.

## chematic API surface used

Confirmed against `chematic 0.12.0` (published on crates.io) by reading
source directly, not guessed. (Upgraded from 0.11.0 — the 0.12.0 changes
are scoped entirely to `chematic-ff`/`chematic-3d`, neither of which
yomitoki's `smiles`/`perception`/`chem` features touch; verified zero output
change across yomitoki's own test suite before and after the bump.)

| Need | chematic API |
|---|---|
| SMILES parsing | `chematic::smiles::parse(&str) -> Result<Molecule, SmilesError>` |
| Molecule struct | `chematic::core::Molecule` (re-exported from `chematic_core`, gated on the `smiles` feature, not its own) |
| Ring perception (SSSR) | `chematic::perception::find_sssr(&Molecule) -> RingSet` |
| Ring system classification | `chematic::perception::find_ring_families(&Molecule, &RingSet) -> Vec<RingFamily>`, `RingFamily.kind: RingSystemKind::{Simple, Fused, Spiro, Bridged}` |
| Valence validation | `chematic::core::validate_valence(&Molecule) -> Vec<ValenceError>` |
| Disconnected fragments | `Molecule::is_connected() -> bool` |
| Stereo completeness | `chematic::perception::stereo_validation::stereo_completeness(&Molecule) -> StereoCompleteness` |
| Molecular weight | `chematic::chem::molecular_weight(&Molecule) -> f64` |
| Rotatable bond count | `chematic::chem::rotatable_bond_count(&Molecule) -> usize` |
| Reactive/unstable functional groups | `chematic::chem::brenk_matches_detailed(&Molecule) -> Vec<(&'static str, Vec<AtomIdx>)>` (Brenk et al. 2008 structural alerts; an entry with an empty atom list means that alert's search was budget-cut, not a zero-atom match) |
| Functional-group clustering | `chematic::chem::identify_functional_groups(&Molecule) -> Vec<FunctionalGroup>` (Ertl 2017; one entry per topologically connected heteroatom-containing environment) |
| SAscore (comparison only, not used by any component) | `chematic::chem::sa_score(&Molecule) -> f64` (Ertl & Schuffenhauer 2009; `examples/sa_score_comparison.rs` only) |
| SDF batch reading | `chematic::mol::SdfReader::new(&str) -> impl Iterator<Item = Result<(Molecule, MolMetadata), MolParseError>>` — CLI-only, gated on the `mol` feature |
| SMILES-table batch reading | `chematic::mol::SmilesRecordReader::new(impl BufRead, SmilesReaderOptions) -> impl Iterator<Item = Result<MoleculeRecord, SmilesTableError>>` — CLI-only, gated on the `mol` feature |
| Circular/ECFP-like fragment hashing | `chematic::fp::morgan_fp_counts(&Molecule, radius: u32) -> HashMap<u64, u32>` (`fragment_rarity`; cumulative over iterations `0..=radius`, gated on the `fp` feature) |

Dependency declaration: `chematic = { version = "0.12", features = ["smiles",
"perception", "chem", "mol", "fp"] }`. The `chematic` facade crate has
`default = []` — without explicit features it exposes nothing. `mol` is used
only by the CLI binary (`src/bin/yomitoki.rs`), not by the library; `fp` is
used only by `fragment_rarity`.

Known gaps in chematic's public API (relevant to yomitoki, not filed upstream
yet): no macrocycle predicate in `chematic-perception` (only
`chematic-3d::detect_macrocycle_status`, gated behind the unrelated `threed`
feature); no single unified `sanitize()`/`validate()` entry point (valence,
stereo, and connectivity checks are independent calls); no single top-level
error type spanning all of chematic's functional areas; no oxidation-state
API anywhere in `chematic-chem`/`chematic-perception` (confirmed absent by
grep, not assumed); `chematic::smarts::find_matches` has no non-overlapping
match mode (every embedding is returned, including overlapping ones) —
irrelevant to `brenk_matches_detailed`, which already reports one entry per
triggered alert rather than per embedding.

## Report schema

```text
SynthesizabilityReport
├── overall: OverallAssessment { synthesizability, difficulty, confidence, verdict }
├── components: ComponentScores        // six Option<ComponentScore> fields
├── findings: Vec<Finding>
├── dominant_penalties: Vec<Contribution>
├── dominant_supports: Vec<Contribution>
├── suggestions: Vec<SimplificationSuggestion>   // populated for 3 of 6 SuggestionCode variants
├── applicability: ApplicabilityReport
└── provenance: Provenance
```

`dominant_penalties` is sorted by each finding's actual contribution weight
(from `ring_topology`/`size_topology`/`stereochemical_burden`/
`functional_group_liability`), not by `Finding.severity` — the two are
deliberately independent axes (severity is
a per-finding chemistry judgment; contribution is what actually fed
`difficulty`). A `Severity::Low` finding can legitimately rank above a
`Severity::High` one if its weight is larger.

`Finding.severity` is currently a fixed, author-assigned constant per
`FindingCode` (set at the call site in each component, e.g.
`Severity::Medium` for `StereoDensityHigh`) — no code in this crate reads it
back. It is part of the public schema for forward compatibility (a future
severity-aware consumer, or a v0.2 that derives it from evidence) but should
not yet be treated as a calibrated signal.

Cross-component ranking in `dominant_penalties` is real coupling: each
component's weight scale (`RING_WEIGHT_BRIDGED`,
`SIZE_WEIGHT_PER_ROTATABLE_BOND`, `STEREO_WEIGHT_PER_CENTER`,
`FG_WEIGHT_PER_REACTIVE_GROUP`) was chosen independently, so a
stereocenter-dense or reactive-group-dense molecule can legitimately
outrank a bridged-ring finding once enough of them pile up — see
`tests/aggregation.rs`'s
`dominant_penalties_rank_across_components_by_contribution_not_by_component_identity`
for a fixture pinning down a specific case of this.

`ComponentScores` has all six report fields
(`size_topology`, `ring_topology`, `stereochemical_burden`, `fragment_rarity`,
`functional_group_liability`, `input_quality`), each typed
`Option<ComponentScore>`. `ring_topology`, `size_topology`,
`stereochemical_burden`, `functional_group_liability`, and `input_quality`
are always `Some` in v0.1. `fragment_rarity` is implemented but opt-in: it's
`Some` only when `AnalysisConfig.fragment_model` has a `FragmentCorpus`
configured, `None` otherwise (the default — no corpus ships with yomitoki
itself; §5.4 below). This is a deliberate choice over populating
unimplemented/unconfigured components with dummy zero scores — `None` says
"not evaluated," a zero score would falsely say "evaluated, found no
burden." Going from `Option` to always-`Some` later is additive; the
reverse would be a breaking schema change, so starting with `Option` is
also the safer long-term default.

`Verdict` defines all six variants (`LikelyAccessible`,
`ModeratelyAccessible`, `Challenging`, `HighlyChallenging`, `Indeterminate`,
`OutOfDomain`) for schema stability, marked `#[non_exhaustive]`. All six are
reachable today (see `analyze::tests` for a unit test per branch): the four
accessibility levels come from the weighted combination of
`ring_topology`/`size_topology`/`stereochemical_burden`/
`functional_group_liability`'s normalized burden, `OutOfDomain` from
applicability's hard-fail triggers, and `Indeterminate` when confidence
falls below a strictness-dependent threshold without an outright hard fail.

`FindingCode` is `#[non_exhaustive]` and currently only defines the codes
the five implemented components actually emit: `RingBridgedComplexity`,
`RingSpiro`, `RingFusedDense`, `RingMacrocycle`, `SizeLargeMolecularWeight`,
`SizeHighRotatableBondCount`, `StereoCenterCount`, `StereoDensityHigh`,
`StereoAnalysisSkipped`, `FunctionalGroupReactive`, `FunctionalGroupDense`,
`InputUnsupportedElement`, `InputDisconnected`, `InputUnusualValence`,
`InputTooLarge`. `FunctionalGroupReactive` is deliberately one generic code
covering every triggered Brenk alert rather than one code per alert (~105
patterns) — the specific alert name is carried in the finding's
`explanation` text and the matched atoms in `atoms`, not in a proliferation
of finding codes. `FunctionalGroupDense` and `StereoAnalysisSkipped` are
both molecule-level findings (empty `atoms`, like `StereoDensityHigh`)
rather than tied to one specific region. Codes for not-yet-implemented
components (e.g. `FRAGMENT_RARE`) are added when those components are.

Every finding's `explanation: String` is generated from its structured code +
parameters (`explain.rs`), never authored by hand per instance — this keeps
structured data as the source of truth and leaves room for future
localization.

## Component interface

Each component module (`components/*.rs`) exposes a `pub(crate) fn compute`
taking `&Molecule` (and `&AnalysisConfig` where config affects the
component, e.g. `max_heavy_atoms`) and returning a `ComponentScore` plus any
component-specific report data. Components do not depend on each other's
output — aggregation happens once, centrally, in `analyze.rs`.

## Simplification suggestions

`suggestions.rs` is a pure function, `derive(findings:
&[Finding]) -> Vec<SimplificationSuggestion>`, run once in `analyze.rs`
after all component findings are collected — not a component itself (it
doesn't contribute to `difficulty`/`confidence`), and it depends only on the
already-finalized `findings` list, not on raw component internals. Suggestions
are derived regardless of `overall.verdict`: a finding is real whether or
not the same molecule is also `OutOfDomain` or `Indeterminate` for an
unrelated reason (e.g. a disconnected fragment), so a suggestion can appear
alongside either verdict — this is a deliberate choice, matching how
`dominant_penalties` already includes every difficulty-contributing finding
regardless of verdict.

4 of `SuggestionCode`'s 6 variants are reachable in v0.1, one per finding
code this module knows how to translate:
`RingBridgedComplexity` → `ReplaceBridgedRingWithMonocyclicAnalog`,
`RingMacrocycle` → `SimplifyMacrocyclicClosure`,
`StereoDensityHigh` → `ReduceStereocenterDensity`,
`FragmentRarityHigh` → `IncreaseFragmentPrecedent` (only reachable when a
fragment corpus is configured — see below). The other 2 have no underlying
signal to derive from yet: quaternary-carbon adjacency isn't computed
anywhere; `brenk_matches_detailed` unions atoms per pattern rather than
reporting per-occurrence matches, so `RemoveSimilarReactiveGroup` can't
identify which specific occurrence to point at.

`target_atoms` is copied directly from the source finding's own `atoms`
field — for `ReduceStereocenterDensity` this is always empty, because
`stereochemical_burden`'s findings never carry atom indices in the first
place. This is a confirmed chematic API gap, not an oversight: chematic's
`stereo_completeness` (used by `stereochemical_burden::compute`) reports
only aggregate counts (`specified`/`unspecified`/`total_centers`), not which
atoms are centers, and `chematic-chem`'s `assign_cip`/
`tetrahedral_stereo_neighbors` only cover atoms with an explicit `@`/`@@`
chirality annotation — they'd under-count relative to the density this
finding is actually about (specified *and* unspecified centers together).

`ExpectedEffect` is always `MayReduceDifficulty`, never
`LikelyReducesDifficulty` — nothing in v0.1 is calibrated against real
synthesis outcomes, so claiming the higher-certainty variant would overstate
what this crate knows. `confidence` is a single flat named constant,
`rules::SUGGESTION_CONFIDENCE_HEURISTIC` (`0.5`), applied to every
suggestion regardless of which finding it came from, for the same reason:
differentiating confidence between suggestion codes without calibration
data would imply a precision this crate doesn't have.

## Scoring direction

* `synthesizability`: 1.0 = easy to make.
* `difficulty`: 1.0 = hard to make.

`difficulty` is a **weighted sum**, not a weighted average, of the four
difficulty-contributing components' normalized scores — an unnormalized sum
of `weight * normalized` terms, not a divide-by-weight-total average:

```text
difficulty = AGGREGATE_WEIGHT_RING_TOPOLOGY * ring_topology.normalized
           + AGGREGATE_WEIGHT_SIZE_TOPOLOGY * size_topology.normalized
           + AGGREGATE_WEIGHT_STEREOCHEMICAL_BURDEN * stereochemical_burden.normalized
           + AGGREGATE_WEIGHT_FUNCTIONAL_GROUP_LIABILITY * functional_group_liability.normalized
```

clamped to `0.0..=1.0` by `ProbabilityLikeScore::new` (weights don't need to
sum to 1). Ring topology's weight is `1.0` — full pass-through — so a
molecule with negligible burden from every other component scores
identically to the single-ring-topology-component model this crate started
with; size, stereo, and functional-group liability all contribute
additively on top at smaller weights, so each registers as extra burden
without diluting a strong ring-topology signal when it itself is small. See
`rules.rs` for the exact weights and the reasoning against a normalized
(divide-by-total) average.

`synthesizability = 1.0 - difficulty`. This complementary relationship is a
v0.1 implementation detail, not a permanent API guarantee — the two fields
may decouple once calibration is introduced.

**Known caveat, documented rather than hidden:** `size_topology`'s
rotatable-bond term over-penalizes simple, commercially available long
unbranched chains (many rotatable bonds, essentially no synthetic
difficulty) — the same "structural complexity vs. actual difficulty"
conflation that existing SA-scoring tools are prone to. `fragment_rarity`
was meant to correct for this by recognizing such fragments as
common/precedented, and is now implemented — but round 16 found it
currently makes this specific case *worse*: dodecane's `overall.
difficulty` goes from `0.068` to `0.227` once a corpus is configured, not
down. Confirmed formula defect, not a missing feature — see "Non-goals /
deferred" below and `rules::FRAGMENT_RARITY_WEIGHT`'s doc comment.

**Second known caveat, same shape:** `functional_group_liability` wraps
Brenk et al. (2008) directly, which was validated as a med-chem
screening-library *desirability* filter, not a synthetic-difficulty
signal. Several of its alerts fire on common, cheaply-precedented groups —
aspirin trips `phenol`/`phenolic_aldehyde`/`active_ester`/`acetal_ketal`
and lands at `ModeratelyAccessible` (synthesizability 0.74) despite being
one of the most trivially synthesizable molecules there is; paracetamol
similarly trips `phenol`/`aniline`/`secondary_amine`. `fragment_rarity`
was expected to correct for this the same way — same result: aspirin's
`overall.difficulty` goes from `0.273` to `0.428` once a corpus is
configured, confirmed end-to-end, not merely predicted.

**Third known caveat, different shape:** `functional_group_liability`'s
"dense functionalization" term (`identify_functional_groups`) counts
topologically *disconnected* functional-group clusters, since Ertl's
algorithm merges any heteroatom-adjacent atoms into one connected
component. Confirmed empirically: glucose (6 hydroxyls on one ring) and
penicillin V (β-lactam + thioether + amide + carboxylic acid + aryl ether,
all ring-fused) both come back as a single cluster — identical in count to
ethanol's lone C-O group. This term only catches *disconnected* multi-site
burden (e.g. several separate esters on one branched core); it has no
substitute-fix planned (unlike the two caveats above, this isn't a
fragment-rarity gap — it's an inherent property of connected-component
clustering as an operationalization of "how many independent reactive
regions require separate synthetic handling").

## Confidence contract

`confidence` comes entirely from the `input_quality`/applicability
component: a product of named per-check penalty factors (element coverage,
valence validity, connectivity, stereo completeness — see `rules.rs`), not a
hand-tuned per-molecule number. Confidence and difficulty are never
conflated — a structurally complex molecule is not automatically
low-confidence; only actual input-quality/applicability problems lower
confidence.

`ring_topology`'s and `size_topology`'s own `ComponentScore.confidence` are
fixed at `1.0`: both are plain deterministic descriptor computations
(`find_ring_families`, `molecular_weight`, `rotatable_bond_count`) for any
molecule that parsed and passed valence validation, so there's no additional
uncertainty to express there yet. `stereochemical_burden`'s is *usually*
`1.0` for the same reason, but drops to `CONFIDENCE_PENALTY_STEREO_
UNCHECKABLE` (0.6) for a molecule with a negatively charged atom — see
"Negatively charged atoms" below.

`functional_group_liability`'s `ComponentScore.confidence` is the minimum
confidence across its own findings, and is usually `1.0` (Brenk pattern
matching is deterministic) — it drops to
`FG_CONFIDENCE_BUDGET_EXHAUSTED` (0.5) only for a finding whose
`brenk_matches_detailed` alert had its VF2 enumeration cut off by the visit
budget before completing (an empty `atom_indices`, still a real flagged
alert per that function's own doc, just one whose match extent is
unresolved). None of these four components' confidence values are wired
into `overall.confidence` yet — only applicability's is — so this is
informational in the schema today, not yet load-bearing for verdict
selection.

`fragment_rarity` is exactly the kind of component whose rule coverage
genuinely varies (with which corpus is configured, and how well it covers
a given molecule), but its `ComponentScore.confidence` is still a flat
`1.0` in v0.1 — deliberately: there's no sampling-uncertainty model yet for
"how much should an unseen fragment's rarity be discounted by corpus size"
(see `components/fragment_rarity.rs`). Confidence will stop being
effectively-constant once that gap is closed, not merely once the
component exists.

## Negatively charged atoms (a chematic bug, worked around)

Found while adding "salts" corpus coverage (AGENTS.md §14.5) to
`tests/property_based.rs`: `chematic::perception::stereo_validation::
stereo_completeness` (called by both `applicability` and
`stereochemical_burden`) computes each atom's initial Morgan-rank
invariant as `atomic_number as u64 * 1_000_000 + charge as u64 * 1000 +
degree`. `Atom.charge` is `i8`; casting any *negative* charge to `u64`
sign-extends before reinterpreting, so the multiplication overflows
unconditionally — panics in debug builds (`attempt to multiply with
overflow`), silently produces a corrupted-but-not-obviously-wrong result
in release builds (confirmed empirically: the one case checked directly,
a charged stereocenter-bearing molecule, still counted correctly, but
that's incidental to wrapping arithmetic, not something the algorithm's
design guarantees). Filed upstream:
[chematic#267](https://github.com/kent-tokyo/chematic/issues/267).

This meant `analyze`/`analyze_smiles`/`analyze_batch` panicked in debug
builds on *any* molecule containing a negatively charged atom — any
carboxylate, sulfonate, phosphate, or other anion, extremely common in
real chemistry — a direct violation of AGENTS.md §27's "panicしない"
completion criterion, undetected for 12 rounds because no test fixture
anywhere in this crate's corpus (fixed lists or `proptest` generators)
had ever included a charged atom.

`components::has_negatively_charged_atom(&Molecule) -> bool` guards both
call sites. When it's true:

* `applicability` never calls `stereo_completeness`. `stereo_complete` is
  `false` (honest — it wasn't checked, not confirmed complete) and a new
  `ApplicabilityReport.stereo_uncheckable` field is `true`, distinguishing
  this from the ordinary "checked, found unspecified centers" case —
  `stereo_complete=false` alone would conflate two findings that call for
  different reader actions. Confidence gets `CONFIDENCE_PENALTY_STEREO_
  UNCHECKABLE` (0.6) instead of `CONFIDENCE_PENALTY_STEREO_INCOMPLETE`
  (0.85) — deliberately a *stronger* penalty, since "zero stereo
  information" is a bigger gap than "checked, some centers unspecified".
  The two are mutually exclusive per molecule.
* `stereochemical_burden` never calls `stereo_completeness` either.
  `total_centers`/density fall back to `0`, but `ComponentScore` stays
  `Some` with a `FindingCode::StereoAnalysisSkipped` finding explaining why
  — a bare zero here would be exactly the "fabricated zero" this project's
  own `ComponentScores` doc explicitly refuses (`None` means "not
  evaluated," a real zero means "evaluated, found none" — this is neither,
  so it needs the finding to stay honest). `ComponentScore.confidence`
  also drops to `CONFIDENCE_PENALTY_STEREO_UNCHECKABLE`, not the usual
  `1.0`.

Not a hard `OutOfDomain` trigger: ring/size/functional-group-liability
scoring all work fine on a charged molecule (none call
`stereo_completeness`), and anionic species are mainstream, legitimate
chemistry — treating every salt as fully out of scope would be far more
punitive than the actual, narrow gap (stereo assessment specifically).
`RULESET_VERSION` bumped to 0.7.0 for the new confidence-penalty constant
and its effect on `overall.confidence` via applicability.

## Abstention contract

`Verdict::OutOfDomain` fires when the applicability component's hard trigger
fires: disconnected fragments, too high a fraction of unsupported elements,
or heavy-atom count above `AnalysisConfig::max_heavy_atoms`. `analyze` still
returns `Ok(report)` in this case with whatever partial diagnostics were
computable — abstention is never an `Err`.

`Verdict::Indeterminate` fires when confidence is below a threshold that
depends on `AnalysisConfig::strictness`
(`rules::indeterminate_confidence_threshold`: 0.3 lenient / 0.45 standard /
0.6 strict) without an outright applicability hard fail. The standard
threshold is deliberately set above the confidence floor applicability's two
soft penalties can reach together (0.5 × 0.85 = 0.425) — a threshold at or
below that floor would make `Indeterminate` unreachable at standard
strictness. See `analyze::tests` for the regression tests covering this.

## Versioning

`Provenance` fields and their sources:

| Field | Source |
|---|---|
| `schema_version` | literal constant in `provenance.rs` (currently `0.2.0`) |
| `yomitoki_version` | `env!("CARGO_PKG_VERSION")` |
| `chematic_version` | chematic's declared version requirement |
| `ruleset_version` | `rules::RULESET_VERSION` |
| `config_hash` | SHA-256 (via the `sha2` crate) of the config's canonical JSON serialization |

`config_hash` deliberately does not use `std::hash::DefaultHasher` — that
hasher is randomized per-process on recent Rust versions and would silently
break both determinism and cross-run provenance comparability.

## Comparison with SAscore

AGENTS.md §27's v0.1 completion criterion ("SAscoreとの最低限の比較結果があ
る") is satisfied by `examples/sa_score_comparison.rs`, an in-process
comparison against `chematic::chem::sa_score` (Ertl & Schuffenhauer 2009).
In-process because `sa_score` is a Rust function in a dependency yomitoki
already has — AGENTS.md §13's `benchmarks/`-external-script requirement is
about *competing Python implementations* (SYBA, SCScore, RAscore), which
this isn't.

Explicitly not a calibration or accuracy claim (that's separate, deferred,
future work — see Non-goals below): the two scores aren't fit against each
other, measure different things (SAscore: fragment frequency + complexity
penalty; yomitoki: structural burden by component), and run on opposite
scales (SAscore `1`..`10`, easy..hard; yomitoki `difficulty` `0.0`..`1.0`,
easy..hard) that the example deliberately does not rescale onto a shared
axis. The value of the comparison is in where the two diverge, not where
they agree — e.g. acyl halide (SAscore `6.62`, yomitoki `0.09`
`LikelyAccessible`) and aspirin (SAscore `4.67`, yomitoki `0.27`
`ModeratelyAccessible`, the same Brenk-validity gap documented in "Scoring
direction" above). Reused the 14-fixture corpus already in
`tests/property_based.rs`'s fixed-molecule arm rather than inventing a
second one.

## Non-goals / deferred

Not implemented in v0.1 so far (tracked, not stubbed with fake data):

* A corpus shipped with (or alongside) yomitoki for `fragment_rarity` to
  use by default. The component itself is implemented
  (`components/fragment_rarity.rs`) and opt-in via
  `AnalysisConfig.fragment_model`, but no corpus ships — AGENTS.md §5.4
  forbids embedding one directly in the library as a huge binary, and no
  decision has been made about the `yomitoki-core`/`yomitoki-models`/
  `yomitoki-data` split (or a feature-flagged external file) §5.4 offers as
  the two alternatives. Build one locally with
  `tools/build-fragment-corpus` and load it with
  `FragmentCorpus::load_dir` in the meantime. See
  `tasks/upstream_and_corpus_research.md` (gitignored) for the corpus-size
  -vs-signal measurements this was decided in view of.
* **`fragment_rarity`'s scoring formula is confirmed broken, not merely
  "unvalidated."** Run end-to-end against a real 200k-molecule ChEMBL
  corpus (round 16): aspirin's `overall.difficulty` went from `0.273` (no
  corpus) to `0.428` (corpus configured) — worse, not better, for exactly
  the case (Brenk-alert over-firing) it exists to correct; dodecane's went
  from `0.068` to `0.227` — same direction of failure for the
  rotatable-bond case. Root cause: `raw = FRAGMENT_RARITY_WEIGHT * (1.0 -
  mean_document_frequency)` has no "common enough → ~zero contribution"
  reference point — real molecules' mean document frequency in a diverse
  corpus rarely exceeds ~0.3–0.4 even for ordinary fragments, so `1.0 -
  mean_document_frequency` sits around `0.7` and contributes positive
  burden for essentially every molecule, common or not. This is a formula
  defect, not a constant to retune — see `rules::FRAGMENT_RARITY_WEIGHT`'s
  doc comment for the full analysis and what a fix would need (a
  reference point relative to the corpus's own distribution, not an
  absolute `1.0` ceiling). `FRAGMENT_RARITY_BURDEN_SCALE` also has a known
  ceiling effect (`normalized` can't exceed ~0.49 with the current
  constants) that's secondary to the formula problem. Confidence is a flat
  `1.0`, with no model yet for how corpus size/coverage should discount
  it. **Until redesigned, configuring a fragment corpus makes the two
  documented false positives worse, not better — this is not currently a
  usable correction mechanism.**
* Candidate `stereochemical_burden` indicators, each investigated and
  rejected/deferred for a distinct, evidenced reason (round 12 — corrects
  an earlier, inaccurate blanket "E/Z needs 2D coordinates" note):
  * **E/Z double-bond stereo** — `chematic::chem::cip::assign_cip` (already
    used elsewhere in this crate) assigns E/Z directly from SMILES `/`/`\`
    bond-direction markers, no 2D coordinates required. But it only covers
    bonds the input SMILES explicitly marked — there is no chematic
    function analogous to `stereo_completeness` that detects a
    stereogenic-but-*unspecified* double bond. A specified-only count would
    violate `STEREO_WEIGHT_PER_CENTER`'s own stated policy (tetrahedral
    centers burden "specified or unspecified... equally," since whether the
    SMILES wrote it out is a confidence concern, not a difficulty one) —
    `C/C=C/C` and `CC=CC` are the same compound and must not get different
    difficulty. Deferred, not implemented as specified-only.
  * **Atropisomerism** — `chematic::chem::detect_atropisomers` exists and
    runs on a plain `Molecule` (no coordinates), but was empirically
    disqualified before use: probed directly against `c1ccccc1-c2ccccc2`
    (flagged as an atropisomer) vs. the same molecule written
    `c1ccccc1c2ccccc2` (not flagged) — a real representation-dependence bug
    that would violate `tests/determinism.rs`'s canonical-SMILES-invariance
    guarantee if wrapped as-is. Also confirmed it rates *para*-substituted
    biphenyl (not sterically hindered) identically to genuinely hindered
    *ortho*-substituted biphenyl — the heuristic (`ipso-carbon degree >= 3`
    on both sides) doesn't actually check substitution position. Not a
    citable/validated primitive; not wrapped.
  * **Contiguous stereocenter runs, quaternary-carbon adjacency** — both
    need an atom-level list of stereocenter candidates (specified and
    unspecified), which chematic only exposes as aggregate counts
    (`stereo_completeness`). The underlying algorithm
    (`simple_morgan_ranks` + 4-distinct-neighbor check, in
    `chematic-perception::stereo_validation`) is `pub(crate)`-only —
    reading and reimplementing it inside yomitoki was considered and
    rejected: every implemented component to date *wraps* a public,
    validated chematic function rather than reimplementing chematic-owned
    perception logic; doing so here for two comparatively low-value
    findings would make yomitoki the second, independent owner of
    stereocenter-candidate detection, with its own correctness burden and
    no chematic test suite backing it. Deferred until chematic exposes the
    atom-level candidates itself.
  * **Meso compound detection** — needs graph automorphism / topological
    symmetry-class computation. chematic has this
    (`chematic-smiles::canonical_automorphism`, `canonical_partition`) but
    both modules are crate-internal (no `pub` items) — nothing to build on
    from outside the crate, and implementing automorphism detection from
    scratch in yomitoki carries the same "second independent owner of
    perception logic" risk as the item above, at higher algorithmic
    complexity.
* Within `functional_group_liability`: dense functionalization is now
  implemented (`identify_functional_groups` cluster count — see the third
  known caveat above for its own gap). Still not implemented: mutually
  incompatible functional-group combinations and protecting-group
  pressure — unlike Brenk (2008) and Ertl (2017), which the two
  implemented liabilities wrap directly, neither has a citable, validated
  primitive to build on (chematic exposes none), and hand-curating either
  would be exactly the "chemically weak rules, over-generalized" AGENTS.md
  §5.5 warns against. Also not implemented: chemoselectivity burden,
  polyfunctional symmetry breaking, multiple-similar-reactive-site
  counting, and difficult oxidation-state combinations. The oxidation-state
  one specifically is blocked on chematic exposing no oxidation-state API
  at all (confirmed absent, not a scope choice); multiple-similar-
  reactive-site counting is blocked on `brenk_matches_detailed` unioning
  atoms per pattern rather than per occurrence; the rest are additive
  future work, contingent on a citable source turning up.
* Simplification suggestions — 3 of `SuggestionCode`'s 6 variants
  (`ReduceAdjacentQuaternaryCenters`, `RemoveSimilarReactiveGroup`,
  `IncreaseFragmentPrecedent`) are not reachable yet; see "Simplification
  suggestions" above for exactly why each is blocked.
* Fragment corpus, model files, calibration, ML.
* `ApplicabilityReport.domain_distance` — needs a calibration corpus that
  doesn't exist yet; always `None`.
* Python/WASM bindings.

Permanent non-goals: retrosynthesis planning, reaction template
application, precursor generation, route ranking, yield prediction,
toxicity/hazard (SDS) classification, cost prediction, full periodic-table
or organometallic/polymer support.
