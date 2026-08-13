# yomitoki architecture (v0.1)

This document defines the crate boundary, public API, report schema, component
interface, scoring direction, confidence/abstention contract, versioning
scheme, and non-goals for yomitoki v0.1. It reflects what is actually
implemented today, not the eventual full scope — see "Non-goals / deferred"
at the end for what's intentionally missing.

yomitoki reads and explains molecular structure — it does not modify,
optimize, or regenerate molecules. That functionality, if it's ever built,
belongs to a different, unrelated project.

**Ecosystem boundary (decided v0.3, round 22 part 23 — see
`benchmarks/synthesizability/v03_two_axis_product_framing/README.md`):**

```text
chematic  -> molecular/reaction primitives
yomitoki  -> intrinsic structural synthesizability
             ("What about this molecule makes synthesis structurally demanding?")
renkin    -> route-dependent planning and evidence
             ("How can we actually make it?")
```

yomitoki computes and reports intrinsic structural synthesizability only.
Route-dependent difficulty (precursor availability, protecting-group
strategy, convergence, reaction precedent, historical route choice) is out
of scope **by design**, established by evidence (PaRoutes final holdout +
semantic ceiling audit: real route length correlates only weakly with any
route-free structural representation tried, while purchasable-stock
similarity — information no single-molecule representation can see —
correlates more strongly in exactly the structurally complex population
where it matters most), not by current limitation. That axis belongs to
[RENKIN](https://github.com/kent-tokyo/renkin), a real, independently
-developed retrosynthesis/CASP project whose own stated scope matches it.
The yomitoki -> RENKIN interface contract (what a report would hand off,
and in what shape) is not yet formalized — open future work, not implied
by this boundary.

## Crate boundary

Single crate, `yomitoki`, no workspace split. `fragment_precedent` now
exists and needs a `FragmentCorpus`, but that corpus is loaded at runtime from an
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
| Circular/ECFP-like fragment hashing | `chematic::fp::morgan_fp_counts(&Molecule, radius: u32) -> HashMap<u64, u32>` (`fragment_precedent`; cumulative over iterations `0..=radius`, gated on the `fp` feature) |

Dependency declaration: `chematic = { version = "0.12", features = ["smiles",
"perception", "chem", "mol", "fp"] }`. The `chematic` facade crate has
`default = []` — without explicit features it exposes nothing. `mol` is used
only by the CLI binary (`src/bin/yomitoki.rs`), not by the library; `fp` is
used only by `fragment_precedent`.

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

`ComponentScores` has five fields (`size_topology`, `ring_topology`,
`stereochemical_burden`, `functional_group_liability`, `input_quality`),
each typed `Option<ComponentScore>`, all always `Some` in v0.1 — every
field in this struct is a difficulty *contributor*, and (round 21) every
one of them always runs. `fragment_precedent` is **not** a field here
— it's implemented and opt-in (`Some` only when
`AnalysisConfig.fragment_model` has a `FragmentCorpus` configured, `None`
otherwise — no corpus ships with yomitoki itself; §5.4 below), but since
round 21 (option C) it no longer contributes to `overall.difficulty`, so
it lives in its own top-level `SynthesizabilityReport.fragment_precedent:
Option<FragmentPrecedentEvidence>` field instead — see "Scoring
direction"'s fourth caveat. `None`-vs-populated is a deliberate choice
over dummy zero scores throughout this crate — `None` says "not
evaluated," a zero score would falsely say "evaluated, found no burden."

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

3 of `SuggestionCode`'s 6 variants are reachable in v0.1, one per finding
code this module knows how to translate:
`RingBridgedComplexity` → `ReplaceBridgedRingWithMonocyclicAnalog`,
`RingMacrocycle` → `SimplifyMacrocyclicClosure`,
`StereoDensityHigh` → `ReduceStereocenterDensity`. The other 3 are
unreachable, each for a different reason: `IncreaseFragmentPrecedent` was
reachable through round 20 but **retired round 21** (option C) — once
`fragment_precedent` stopped contributing to `overall.difficulty`, "a more
precedented analog would lower this contribution to difficulty" became a
false claim, so `suggestions.rs` deliberately has no match arm for
`FragmentPrecedentWeak` anymore (kept in the `SuggestionCode` schema since
it's `#[non_exhaustive]`, never emitted). `ReduceAdjacentQuaternaryCenters`/
`RemoveSimilarReactiveGroup` still have no underlying signal to derive from:
quaternary-carbon adjacency has an atom-level candidate list now
(`stereo_centers`, chematic 0.13.0 — see "Non-goals / deferred" below) but
no adjacency-detection rule or `FindingCode` built on it yet; `RemoveSimilarReactiveGroup`
is separately blocked on `brenk_matches_detailed` unioning atoms per pattern
rather than reporting per-occurrence matches, so it still can't identify
which specific occurrence to point at.

`target_atoms` is copied directly from the source finding's own `atoms`
field. For `ReduceStereocenterDensity` this used to always be empty,
because `stereochemical_burden`'s findings never carried atom indices in
the first place — through chematic 0.12 this was a real API gap:
`stereo_completeness` (used by `stereochemical_burden::compute`) reported
only aggregate counts (`specified`/`unspecified`/`total_centers`), not
which atoms are centers, and `chematic-chem`'s `assign_cip`/
`tetrahedral_stereo_neighbors` only cover atoms with an explicit `@`/`@@`
chirality annotation — they'd under-count relative to the density this
finding is actually about (specified *and* unspecified centers
together). **Update, chematic 0.13.0 (round 22 part 6):**
`chematic_perception::stereo_centers(&Molecule) -> Vec<(AtomIdx, bool)>`
reports exactly this, specified or not — `stereochemical_burden::compute`
now populates `StereoCenterCount`/`StereoDensityHigh` findings' `atoms`
from it directly, so `ReduceStereocenterDensity`'s `target_atoms` carries
real atom indices. Purely additive: no scoring, weight, or threshold
changed — only previously-empty `atoms`/`target_atoms` fields are now
populated. See `tests/stereochemical_burden.rs`'s
`stereo_findings_now_carry_real_atom_indices`/
`reduce_stereocenter_density_suggestion_now_targets_real_atoms`.

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

Both describe **intrinsic structural difficulty** — burden explainable
from the target molecule alone — not predicted route-dependent
difficulty. Neither field estimates real synthetic step count, precursor
availability, or retrosynthetic search outcome; see the ecosystem
-boundary note above. This is a semantic-contract clarification (v0.3,
round 22 part 23), not a field rename — `overall.difficulty`/
`overall.synthesizability` keep their current names and `schema_version`
is unchanged (still `0.6.0`); a rename is deferred to a future
major/API-redesign decision, since `report.rs`'s own doc comment already
treats the current names as "an implementation choice, not a permanent
API guarantee" (see "Versioning" below).

`difficulty` is a **weighted sum**, not a weighted average, of exactly these
four components' normalized scores — an unnormalized sum of `weight *
normalized` terms, not a divide-by-weight-total average, and (since round
21 — see the fourth caveat below) not a partial description with a
correction term bolted on elsewhere in the code:

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

`fragment_precedent` is a fifth, always-computed signal (when a corpus is
configured) that is **not** part of this sum — see the fourth caveat below
for why, and `SynthesizabilityReport.fragment_precedent`/
`FragmentPrecedentEvidence` for where it actually appears in a report.

`synthesizability = 1.0 - difficulty`. This complementary relationship is a
v0.1 implementation detail, not a permanent API guarantee — the two fields
may decouple once calibration is introduced.

**Known caveat, documented rather than hidden:** `size_topology`'s
rotatable-bond term over-penalizes simple, commercially available long
unbranched chains (many rotatable bonds, essentially no synthetic
difficulty) — the same "structural complexity vs. actual difficulty"
conflation that existing SA-scoring tools are prone to. Rounds 17-20 had
`fragment_precedent` correct for this by recognizing such fragments as
common/precedented and feeding a capped adjustment into `overall.difficulty`
(dodecane's difficulty measured `0.068 → 0.000` against a real 200k
-molecule ChEMBL corpus). **Round 21 removed that correction mechanism
entirely** (option C — see the fourth caveat below): `fragment_precedent`
no longer adjusts `overall.difficulty` at all, so dodecane's difficulty is
back to the uncorrected `0.068` regardless of which corpus is configured.
This over-penalization is therefore an **acknowledged, currently
-unaddressed limitation** of `size_topology`'s rotatable-bond term, not a
solved problem — `fragment_precedent` still reports (as explanatory
evidence) that such chains are strongly precedented, so a report reader
can see the mismatch, but the score itself no longer reflects it. See
`rules.rs`'s "Fragment precedent" section for the full round 16-21
history.

**Second known caveat, same shape:** `functional_group_liability` wraps
Brenk et al. (2008) directly, which was validated as a med-chem
screening-library *desirability* filter, not a synthetic-difficulty
signal. Several of its alerts fire on common, cheaply-precedented groups —
aspirin trips `phenol`/`phenolic_aldehyde`/`active_ester`/`acetal_ketal`
and lands at `ModeratelyAccessible` (synthesizability 0.74) despite being
one of the most trivially synthesizable molecules there is — the same
score with or without a corpus configured, since round 21;
paracetamol similarly trips `phenol`/`aniline`/`secondary_amine`. Through
round 20, `fragment_precedent` corrected this the same way as the
rotatable-bond case above (aspirin `0.273 → 0.095`); since round 21 it no
longer does, for the same reason — see the fourth caveat.

**Fourth known caveat, discovered by round 17's validation panel, extended
by round 19's cross-corpus validation:** `fragment_precedent`'s
corpus-relative signal is only as good as the corpus it's given (round 18
makes this traceable per-report via
`Provenance.fragment_corpus.synthesis_focused`). Against the real
200k-molecule ChEMBL-37 corpus, several
structurally-legitimate molecules score *harder*, not easier, once the
corpus is configured — caffeine (`0.287 → 0.516`), a bridged bicyclic /
norbornane (`0.341 → 0.985`), a spiro ring system (`0.199 → 1.000`), and a
stereocenter-dense molecule (`0.275 → 1.000`). Checked and confirmed not a
formula or cap bug: independently re-deriving each molecule's raw mean
document frequency and percentile from the corpus's own
`fragment_frequencies.json`/`reference_distribution` reproduces the same
numbers the formula uses. The likely cause is corpus-domain bias: ChEMBL
is a bioactivity-screening corpus, so its fragment-frequency table
reflects what shows up in bioassay-tested compounds, not what's a common
synthetic building block — "rare in ChEMBL" and "hard to synthesize" are
different claims.

Round 19 tested the natural fix — a synthesis-focused reference corpus
(Open Reaction Database, matched to ChEMBL's 200k size, leakage-controlled)
— and found the caveat only *partially* corpus-fixable: norbornane's
penalty fell sharply (`p=0.178 → p=0.455`, verdict
`HighlyChallenging → ModeratelyAccessible`), but spiro-decane and
stereocenter-dense stayed almost maximally penalized in both corpora
(`p=0.025 → p=0.061`, `p=0.008 → p=0.007`), and caffeine got *more*
penalized under the synthesis-focused corpus, not less
(`p=0.386 → p=0.048`, signal `+0.23 → +0.91`) — plausibly because caffeine
is mostly obtained by extraction/purchase rather than reported as a
documented reaction product, while it's ubiquitous as a *screened*
bioactive compound in ChEMBL, but that is a per-molecule post-hoc
explanation, not a predictive rule. Reported honestly rather than tuned
away either time: no formula or weight change was made in response to
either round's numbers, and the same corpus-relative framing
("relative to the configured reference corpus," never "generally easy/hard
to synthesize") stays the accurate description of what the signal actually
answers. See `rules.rs`'s "Fragment precedent" section for the full
numbers and `tasks/upstream_and_corpus_research.md` Part 6 (gitignored)
for the corpus-build methodology.

Round 20 settled the question this caveat had left open: is a bounded
amount of corpus sensitivity acceptable, or does it undermine the
contract entirely? Tested with a second synthesis-focused corpus (SynRXN)
and — the change from rounds 17–19 — a 500-molecule *generated* probe
panel rather than the same small diagnostic set. Result: ORD and SynRXN
disagree with each other on precedent direction more often than either
disagrees with ChEMBL (34.6% sign-flip rate over 500 probes), and plain
pyridine — no plausible synthetic-difficulty story available — scores
`HighlyChallenging` under ORD and `LikelyAccessible` under ChEMBL/SynRXN,
driven entirely by `fragment_precedent`'s uncapped penalty term (the other
four components sum to `0.119`). This caveat is no longer "documented and
monitored" — it's now the reason `fragment_precedent` is recommended for
removal from `overall.difficulty` (see the roadmap's item 4 and `rules.rs`
for the full reasoning).

**Round 21 implements that removal (option C), resolving this caveat
structurally rather than by further correction.** `fragment_precedent` no
longer contributes to `overall.difficulty` in any way — confirmed
end-to-end, not just by code inspection: re-ran the same 15-molecule panel
and the round-20 500-probe panel against ChEMBL/ORD/SynRXN with the new
code, and `overall.difficulty` is bit-for-bit identical across all three
corpora (and the no-corpus default) for every one of the 515 molecules
tested, while `fragment_precedent.signed_signal` still genuinely differs
per corpus for 499/500 probes — the signal is untouched, only its
influence on scoring is gone. Pyridine now scores `LikelyAccessible`
(`difficulty=0.1045`) identically regardless of which corpus is
configured. The corpus-domain-bias caveat itself doesn't disappear —
`fragment_precedent` is still exactly as corpus-sensitive as rounds 19-20
found it to be — but it can no longer reach `overall.difficulty`, so it's
no longer a scoring risk, only an explanatory-evidence characteristic (see
`SynthesizabilityReport.fragment_precedent`). The tradeoff, stated
plainly: this reopens the first two known caveats above (aspirin/
dodecane-shaped over-penalization by `size_topology`/
`functional_group_liability`) rather than solving them — round 21
resolves the *riskier* problem (a corpus-dependent, unbounded scoring
adjustment that could swing a verdict) at the cost of reintroducing the
*milder* one (a bounded, always-consistent over-penalization that
doesn't depend on which corpus happens to be configured). See `rules.rs`'s
"Fragment precedent" section for the implementation and
`tasks/upstream_and_corpus_research.md` Part 7/8 (gitignored) for the full
round 20/21 data.

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

`fragment_precedent` is exactly the kind of component whose rule coverage
genuinely varies (with which corpus is configured, and how well it covers
a given molecule), but its `FragmentPrecedentEvidence.confidence` (not a
`ComponentScore` since round 21 — see "Scoring direction"'s fourth
caveat) is still a flat `1.0` in v0.1 — deliberately: there's no
sampling-uncertainty model yet for "how much should an unseen fragment's
rarity be discounted by corpus size" (see `components/fragment_precedent.rs`).
Moot for `overall.confidence` either way — that field comes entirely from
`input_quality`, never from `fragment_precedent`, before or after round
21.

## Negatively charged atoms (a chematic bug, worked around, then fixed upstream)

Found while adding "salts" corpus coverage (AGENTS.md §14.5) to
`tests/property_based.rs`: `chematic::perception::stereo_validation::
stereo_completeness` (called by both `applicability` and
`stereochemical_burden`) computed each atom's initial Morgan-rank
invariant as `atomic_number as u64 * 1_000_000 + charge as u64 * 1000 +
degree`. `Atom.charge` is `i8`; casting any *negative* charge to `u64`
sign-extends before reinterpreting, so the multiplication overflowed
unconditionally — panicked in debug builds (`attempt to multiply with
overflow`), silently produced a corrupted-but-not-obviously-wrong result
in release builds (confirmed empirically: the one case checked directly,
a charged stereocenter-bearing molecule, still counted correctly, but
that was incidental to wrapping arithmetic, not something the
algorithm's design guaranteed). Filed upstream:
[chematic#267](https://github.com/kent-tokyo/chematic/issues/267).

This meant `analyze`/`analyze_smiles`/`analyze_batch` panicked in debug
builds on *any* molecule containing a negatively charged atom — any
carboxylate, sulfonate, phosphate, or other anion, extremely common in
real chemistry — a direct violation of AGENTS.md §27's "panicしない"
completion criterion, undetected for 12 rounds because no test fixture
anywhere in this crate's corpus (fixed lists or `proptest` generators)
had ever included a charged atom.

### The workaround (rounds 12–21, removed round 22)

`components::has_negatively_charged_atom(&Molecule) -> bool` guarded both
call sites. When it was true:

* `applicability` never called `stereo_completeness`. `stereo_complete` was
  `false` (honest — it wasn't checked, not confirmed complete) and
  `ApplicabilityReport.stereo_uncheckable` was `true`, distinguishing
  this from the ordinary "checked, found unspecified centers" case —
  `stereo_complete=false` alone would have conflated two findings that
  call for different reader actions. Confidence got a dedicated,
  stronger penalty (0.6, vs. 0.85 for "checked, some centers
  unspecified") — the two were mutually exclusive per molecule.
* `stereochemical_burden` never called `stereo_completeness` either.
  `total_centers`/density fell back to `0`, but `ComponentScore` stayed
  `Some` with a `FindingCode::StereoAnalysisSkipped` finding explaining
  why — a bare zero there would have been exactly the "fabricated zero"
  this project's own `ComponentScores` doc explicitly refuses.

Not a hard `OutOfDomain` trigger: ring/size/functional-group-liability
scoring all worked fine on a charged molecule (none call
`stereo_completeness`), and anionic species are mainstream, legitimate
chemistry — treating every salt as fully out of scope would have been far
more punitive than the actual, narrow gap (stereo assessment
specifically). `RULESET_VERSION` was bumped to 0.7.0 at the time for the
new confidence-penalty constant and its effect on `overall.confidence`
via applicability.

### The fix (round 22 part 5)

chematic 0.13.0 (2026-08-10) fixed the overflow directly: the invariant
is now computed in `i64` and cast to `u64` once at the end, bit-for-bit
identical to the old code for non-negative charges. Verified directly,
not assumed from the changelog alone — a standalone probe against
chematic 0.13.0 confirmed alaninate (`C[C@@H](N)C(=O)[O-]`, this
project's own long-standing worked example for this bug) now returns
`specified=1, unspecified=0`, matching its neutral-acid form exactly, no
panic.

The workaround was removed the same round: `has_negatively_charged_atom`
deleted, both call sites now call `stereo_completeness` unconditionally,
`CONFIDENCE_PENALTY_STEREO_UNCHECKABLE` removed (no remaining trigger,
`RULESET_VERSION` bumped `0.9.0` → `0.10.0` per this module's own
"bump whenever any constant changes" contract).
`ApplicabilityReport.stereo_uncheckable` and
`FindingCode::StereoAnalysisSkipped` stay in the schema (never removed,
this project's standard compatibility policy for retired-but-not-removed
fields/codes) but are permanently `false`/unreachable now, documented as
such at their definitions rather than silently left describing dead
behavior.

**A real downstream consequence, flagged rather than silently
absorbed**: `overall.confidence`'s achievable floor rose from 0.3
(`CONFIDENCE_PENALTY_UNUSUAL_VALENCE * CONFIDENCE_PENALTY_STEREO_
UNCHECKABLE = 0.5 * 0.6`) to 0.425 (`CONFIDENCE_PENALTY_UNUSUAL_VALENCE *
CONFIDENCE_PENALTY_STEREO_INCOMPLETE = 0.5 * 0.85`), since the lower
-floor combination no longer exists. `Standard` and `Strict` strictness's
`Indeterminate` thresholds (0.45, 0.6) are unaffected (still above the
new floor). `Lenient`'s threshold (0.3) was calibrated against the
now-removed lower floor and is currently unreachable — `Indeterminate`
cannot fire at `Lenient` strictness via applicability penalties alone
anymore. Not silently recalibrated: flagged as an explicit open question
this round, investigated and decided the next round (round 22 part 6,
NO-GO — kept at 0.3). See
`rules::INDETERMINATE_CONFIDENCE_THRESHOLD_LENIENT`'s doc comment for
the full reasoning: the ordering contract (`Lenient` <=
`Standard` <= `Strict`) never actually broke, and with only four
confidence values reachable (`{1.0, 0.85, 0.5, 0.425}`, the product of
two independent binary penalties), no threshold value both changes
`Lenient`'s current behavior *and* keeps it distinct from `Standard` —
any value that fires at all collapses `Lenient` onto exactly the same
case `Standard` already catches.

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
strictness, guarded directly by
`analyze::tests::standard_threshold_stays_above_the_achievable_confidence_floor`.
`Lenient`'s own threshold (0.3) is currently below that floor and
therefore unreachable in practice — investigated as a recalibration
candidate and deliberately kept as-is (round 22 part 6, NO-GO; see the
"Negatively charged atoms" section above and
`rules::INDETERMINATE_CONFIDENCE_THRESHOLD_LENIENT`'s doc comment for the
full reasoning). See `analyze::tests` for the regression tests covering
this.

## Versioning

`Provenance` fields and their sources:

| Field | Source |
|---|---|
| `schema_version` | literal constant in `provenance.rs` (currently `0.6.0`) |
| `yomitoki_version` | `env!("CARGO_PKG_VERSION")` |
| `chematic_version` | chematic's declared version requirement |
| `ruleset_version` | `rules::RULESET_VERSION` |
| `fragment_corpus` | `Some(FragmentCorpusProvenance)` built from the configured corpus's own manifest (`corpus_domain`, `fragment_definition_version`, `reference_distribution_version`) when `AnalysisConfig.fragment_model.corpus` is set, `None` otherwise (round 18) |
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
direction" above — through round 20 a configured corpus dropped this to
`0.095`; since round 21 (option C) `overall.difficulty` no longer changes
with corpus configuration at all, see the fourth caveat there).
Reused the 14-fixture corpus already in
`tests/property_based.rs`'s fixed-molecule arm rather than inventing a
second one.

## Non-goals / deferred

**Roadmap to a non-alpha `0.1.0`, decided after round 17's redesign shipped
as `0.1.0-alpha.2`:** the code/formula/cap design was judged v0.1-ready at
the time, pending corpus-*semantics* work (items 1–3 below); round 20's
item 4 found that judgment was premature — the design has a real gap
(the uncapped penalty side) that only surfaced once tested against a
second corpus and a broad probe panel, not a corpus-semantics question at
all. Round 21's item 5 resolved it. Five items, in order:

1. ~~**Rename `fragment_rarity` to `fragment_precedent`**~~ — **done,
   round 18.** Component module (`components/fragment_precedent.rs`),
   `ComponentScores.fragment_precedent` field, `FindingCode::
   FragmentPrecedentWeak` (was `FragmentRarityHigh`), explanation text,
   suggestions, CLI help, and every doc comment referencing the old name.
   `AnalysisConfig.fragment_model`/`FragmentModelConfig` were audited and
   *not* renamed — see config.rs's own doc comment for the reasoning
   (that name was already generic, not rarity-specific, and renaming it
   to something precedent-specific would narrow it unnecessarily).
   Justification for the parts that did rename: the component no longer
   detects rarity as a one-directional penalty — round 17 made it argue
   difficulty both up (weakly precedented fragments) *and* down (strongly
   precedented ones), so "rarity detector" undersold what it actually
   does. `schema_version` bumped `0.4.0` → `0.5.0`; no deprecated alias
   kept (clean break, pre-`0.1.0`).
2. ~~**A corpus-domain provenance contract in the manifest**~~ — **done,
   round 18.** `manifest.json` now carries a required `corpus_domain`
   block (`source_name`, `domain`, `synthesis_focused`, `description`),
   set via new required `tools/build-fragment-corpus` CLI flags
   (`--corpus-domain-name`/`--corpus-domain`/
   `--corpus-synthesis-focused`/`--corpus-domain-description` — required,
   not defaulted, since guessing a domain would defeat the point). Every
   report produced against a configured corpus now carries this in
   `Provenance.fragment_corpus` (`FragmentCorpusProvenance`, replacing the
   old bare `model_version: Option<String>`), so a report reader can trace
   which corpus *and which domain* produced its `fragment_precedent`
   signal — "rare in ChEMBL" and "hard to synthesize" are traceably
   different claims now, not implicitly conflated. Deliberately
   provenance-only this round: `synthesis_focused: false` does not lower a
   score, reduce confidence, or refuse the corpus — see
   `FragmentCorpusProvenance::synthesis_focused`'s own doc. ChEMBL 37 is
   documented by ChEMBL itself as a bioactive/drug-like-molecule corpus,
   not a synthesis-focused one — round 17's caffeine/norbornane/spiro/
   stereocenter-dense findings are exactly this mismatch surfacing, not a
   formula bug (see "Scoring direction" above); the local `out_200000_v4`
   corpus build declares `synthesis_focused: false` accordingly.
3. ~~**Validate against at least one synthesis-focused reference corpus**~~
   — **done, round 19. Result: CONDITIONAL GO.** Built a matched-size
   (200,000-molecule) corpus from the Open Reaction Database (CC-BY-SA-4.0,
   local-validation-only — its share-alike license means the generated
   artifact must not be bundled into this MIT/Apache-licensed crate;
   distributing an ORD-derived corpus is a separate, undecided question)
   and re-ran the validation panel leakage-controlled against both it and
   ChEMBL. The `GeneralOrganic` profile keeps `fragment_precedent` in
   `overall.difficulty` (option A, unchanged) — the common/simple panel
   never regressed, `ring_topology`/`stereochemical_burden` burden is
   provably preserved regardless of which corpus is configured (confirmed
   directly, not just argued: a molecule's `ring_topology.contribution` is
   bit-for-bit identical across corpora), and no formula/weight was
   retuned to produce this result. Conditional because the result wasn't
   uniformly reassuring: swapping to a synthesis-focused corpus relieved
   some divergent cases (norbornane) but not others (spiro-decane,
   stereocenter-dense stayed near-maximally penalized; caffeine got
   *more* penalized, not less) — see `rules.rs`'s "Fragment precedent"
   section and `tasks/upstream_and_corpus_research.md` Part 6 (gitignored)
   for the full numbers. The corpus-domain-bias caveat is now confirmed
   empirically heterogeneous, not resolved by "use a better corpus" alone
   — documented as an open, corpus-choice-sensitive property of the signal,
   not a blocker to re-litigate before every release.
4. ~~**Robustness-check option A against a second corpus and a broad
   (not hand-picked) probe panel**~~ — **done, round 20. Result:
   supersedes item 3's CONDITIONAL GO — NO-GO for `v0.1.0` as currently
   wired, recommended contract C.** Built a second synthesis-focused
   corpus (SynRXN v0.0.8, USPTO-rooted like most of ORD — an intentional
   preprocessing/curation-robustness test, not a second-domain test) and
   a 500-molecule *generated*, corpus-independent probe panel (never
   sampled from ChEMBL/ORD/SynRXN). Result: ORD and SynRXN — sharing 83%
   of SynRXN's own molecules — disagree with each other on
   `fragment_precedent`'s penalty/support direction 34.6% of the time
   (Spearman ρ=0.48 on `signed_signal`), *worse* agreement than either has
   with ChEMBL (ρ=0.58 and ρ=0.98 respectively). Clearest single case,
   verified by direct fragment-level query, not inferred: plain pyridine
   scores `HighlyChallenging` (`difficulty=1.0`) against ORD and
   `LikelyAccessible` (`difficulty=0.095`) against ChEMBL or SynRXN, with
   `ring_topology`/`size_topology`/`stereochemical_burden`/
   `functional_group_liability` summing to only `0.119` — the entire
   swing is `fragment_precedent`'s own uncapped penalty term.
   **Recommended contract: C** — remove `fragment_precedent` from
   `overall.difficulty`, keep it as explanatory-only evidence. Not
   implemented this round (evaluation round, formula/cap changes
   explicitly out of scope); tracked as item 5's blocker.
   Full methodology, the SynRXN corpus build, the
   overlap audit, and the pyridine mechanism check are in
   `tasks/upstream_and_corpus_research.md` Part 7 (gitignored); the
   durable summary is in `rules.rs`'s "Fragment precedent" section.
5. ~~**Implement option C**~~ — **done, round 21.** `fragment_precedent`
   removed from `overall.difficulty`'s aggregation entirely
   (`analyze::analyze`'s `difficulty_value` is now exactly the four-term
   weighted sum in "Scoring direction" above, unconditionally — no
   correction term, no cap, configured corpus or not). The signal itself
   is unchanged and still fully reported, moved to a new top-level
   `SynthesizabilityReport.fragment_precedent: Option<FragmentPrecedentEvidence>`
   field, structurally incapable of reaching `dominant_penalties`/
   `dominant_supports` (`dominant_supports` is consequently always empty
   in v0.1 now, kept in the schema for a future support-flavored
   component). `SuggestionCode::IncreaseFragmentPrecedent` retired
   (unreachable — "would lower this contribution to difficulty" stopped
   being true). Verified end-to-end against real ChEMBL/ORD/SynRXN
   corpora, not just unit-tested: `overall.difficulty` is bit-for-bit
   identical across all three corpora (and the no-corpus default) for
   every one of the round-19/20 15-molecule panel and 500-probe panel
   molecules, while `fragment_precedent.signed_signal` still genuinely
   differs per corpus for 499/500 probes — pyridine now scores
   `LikelyAccessible` regardless of configured corpus. `schema_version`
   bumped `0.5.0` → `0.6.0`; no deprecated alias (clean break,
   pre-`0.1.0`). **Tradeoff, not a free win:** this reopens the first two
   known caveats in "Scoring direction" above (`size_topology`/
   `functional_group_liability` over-penalizing common building blocks
   like dodecane/aspirin) rather than solving them — round 21 trades the
   *riskier* problem (a corpus-dependent, unbounded scoring adjustment
   that could swing a verdict) for the *milder*, corpus-invariant one.
   See `rules.rs`'s "Fragment precedent" section and
   `tasks/upstream_and_corpus_research.md` Part 8 (gitignored) for the
   full verification data and the resulting v0.1.0 verdict.

Items 1–3 didn't change the underlying formula or cap logic itself, which
round 17 validated end-to-end for the cases it tested. Item 4 found that
validation incomplete, not wrong: the formula behaves exactly as
specified in every case checked, but "behaves as specified" and "safe to
score with" turned out to be different claims once the penalty side's
lack of a cap was stress-tested against corpus-breadth variation. Item 5
resolved this by removing the correction mechanism rather than further
tuning it — see `rules.rs`'s "Fragment precedent" section for the full
reasoning and tradeoff.

**All five roadmap items are done as of round 21 — v0.1.0 verdict: GO.**
No further corpus-semantics or scoring-contract work is a stated blocker;
what remains before actually cutting `0.1.0` (version bump, CHANGELOG
finalization, tag, `cargo publish`) is release mechanics, not open design
questions, and none of it was performed this round (explicitly out of
scope — see the round-21 completion record in
`tasks/upstream_and_corpus_research.md` Part 8, gitignored, for the full
verification this verdict is based on).

Not implemented in v0.1 so far (tracked, not stubbed with fake data):

* A corpus shipped with (or alongside) yomitoki for `fragment_precedent` to
  use by default. The component itself is implemented
  (`components/fragment_precedent.rs`) and opt-in via
  `AnalysisConfig.fragment_model`, but no corpus ships — AGENTS.md §5.4
  forbids embedding one directly in the library as a huge binary, and no
  decision has been made about the `yomitoki-core`/`yomitoki-models`/
  `yomitoki-data` split (or a feature-flagged external file) §5.4 offers as
  the two alternatives. Build one locally with
  `tools/build-fragment-corpus` and load it with
  `FragmentCorpus::load_dir` in the meantime. See
  `tasks/upstream_and_corpus_research.md` (gitignored) for the corpus-size
  -vs-signal measurements this was decided in view of.
* **`fragment_precedent`'s scoring formula was confirmed broken in round
  16, redesigned and fixed in round 17.** Round 16 found `raw =
  FRAGMENT_RARITY_WEIGHT * (1.0 - mean_document_frequency)` (the constant
  itself has since been removed) had no
  "common enough → ~zero contribution" reference point — real molecules'
  mean document frequency in a diverse corpus rarely exceeds ~0.3–0.4 even
  for ordinary fragments, so `1.0 - mean_document_frequency` sat around
  `0.7` and contributed positive burden for essentially every molecule,
  common or not (aspirin `0.273 → 0.428`, dodecane `0.068 → 0.227`, both
  worse once a corpus was configured). Round 17 replaced it with a
  corpus-relative signed-precedent formula: convert a molecule's mean
  document frequency to `p`, its empirical percentile within the
  corpus's own distribution (`FragmentCorpus::percentile_rank`, built from
  a 1001-point quantile grid computed during corpus build), then
  `signed_signal = 1.0 - 2.0 * p` — negative (support) above the median,
  positive (penalty) below it. Precedent support is capped in
  `analyze::analyze` at `size_topology`'s plus
  `functional_group_liability`'s own contribution, so strong fragment
  precedent can never erase `ring_topology`/`stereochemical_burden`
  burden. Confirmed end-to-end against the real 200k-molecule corpus:
  aspirin `0.273 → 0.095`, paracetamol `0.243 → 0.095`, dodecane
  `0.068 → 0.000` — all three documented target cases now move in the
  intended direction. See `rules.rs`'s "Fragment precedent" section for the
  full formula, the measured corpus quantiles it was designed against, and
  its own known caveat: some structurally-legitimate molecules (caffeine,
  bridged/spiro ring systems, stereocenter-dense cores) score *harder*
  against the ChEMBL corpus specifically, due to corpus-domain bias, not a
  formula defect — see "Scoring direction" above for the specific numbers.
  Confidence is still a flat `1.0`, with no model yet for how corpus
  size/coverage should discount it.
* Candidate `stereochemical_burden` indicators, each investigated and
  rejected/deferred for a distinct, evidenced reason (round 12 — corrects
  an earlier, inaccurate blanket "E/Z needs 2D coordinates" note):
  * **E/Z double-bond stereo** — `chematic::chem::cip::assign_cip` (already
    used elsewhere in this crate) assigns E/Z directly from SMILES `/`/`\`
    bond-direction markers, no 2D coordinates required. Originally deferred
    because chematic exposed no function analogous to `stereo_completeness`
    that detects a stereogenic-but-*unspecified* double bond, and a
    specified-only count would violate `STEREO_WEIGHT_PER_CENTER`'s own
    stated policy (tetrahedral centers burden "specified or unspecified...
    equally," since whether the SMILES wrote it out is a confidence
    concern, not a difficulty one — `C/C=C/C` and `CC=CC` are the same
    compound and must not get different difficulty). **Update, chematic
    0.13.0 (round 22 part 5, issue #264):**
    `chematic_chem::ez_completeness(&Molecule) ->
    EzCompleteness{specified, unspecified, total}` now exists, reporting
    all three counts separately — the missing detector. This resolves the
    *data-availability* half of the deferral (burden could weight `total`,
    matching tetrahedral stereocenters' own policy), but not automatically
    the *design* half — still needs a real design pass (weight, threshold,
    interaction with existing tetrahedral burden) before implementing, not
    assumed resolved just because the API exists. See `ROADMAP.md`'s
    "Needs scoping" section (gitignored).
  * **Atropisomerism** — `chematic::chem::detect_atropisomers` exists and
    runs on a plain `Molecule` (no coordinates), but was empirically
    disqualified before use (round 12): probed directly against
    `c1ccccc1-c2ccccc2` (flagged as an atropisomer) vs. the same molecule
    written `c1ccccc1c2ccccc2` (not flagged) — a real
    representation-dependence bug that would violate `tests/determinism.rs`'s
    canonical-SMILES-invariance guarantee if wrapped as-is. Also confirmed
    it rates *para*-substituted biphenyl (not sterically hindered)
    identically to genuinely hindered *ortho*-substituted biphenyl — the
    heuristic (`ipso-carbon degree >= 3` on both sides) doesn't actually
    check substitution position. Not a citable/validated primitive; not
    wrapped. **Update, chematic 0.13.0 (round 22 part 5, issues
    #262/#276):** `detect_atropisomers` was rewritten to be independent of
    inter-ring bond notation — exactly the defect above. **Re-verified
    empirically** (standalone probe against chematic 0.13.0 directly, same
    methodology as the #267 verification — not just trusting the
    changelog), against this project's exact original disqualifying cases:
    `c1ccccc1-c2ccccc2` and `c1ccccc1c2ccccc2` now both report 0 hits
    (notation-invariance defect resolved — and arguably the more
    chemically correct answer too, since unsubstituted biphenyl's rotation
    barrier is low enough to freely rotate at room temperature).
    2,2'-dimethylbiphenyl (*ortho*, sterically hindered) reports 1 hit
    (`Biaryl`); 4,4'-dimethylbiphenyl (*para*, unhindered) reports 0 —
    the position-blindness defect also appears resolved, not just the
    notation one. **Both originally-documented defects are gone**, but
    this was a targeted re-test of the two known failure cases, not a
    full re-validation of the heuristic's accuracy across the space of
    hindered/unhindered biaryls — and `detect_atropisomers` still isn't
    wrapped as a yomitoki component: doing so would add a new burden
    signal (weight, threshold, `FindingCode`), which is a scoring-formula
    decision under the same root-cause-first discipline as the round 22
    external-benchmark findings above, not a mechanical unblock. See
    `ROADMAP.md`'s "Needs scoping" section (gitignored).
  * **Contiguous stereocenter runs, quaternary-carbon adjacency** —
    originally deferred as one item: both need an atom-level list of
    stereocenter candidates (specified and unspecified), which chematic
    only exposed as aggregate counts (`stereo_completeness`); the
    underlying algorithm (`simple_morgan_ranks` + 4-distinct-neighbor
    check, in `chematic-perception::stereo_validation`) was
    `pub(crate)`-only, and reimplementing chematic-owned perception logic
    inside yomitoki was rejected (every implemented component wraps a
    public, validated chematic function rather than owning perception
    logic independently). **Update, chematic 0.13.0 (round 22 part 5,
    issue #263):** `chematic_perception::stereo_centers(&Molecule) ->
    Vec<(AtomIdx, bool)>` now exposes exactly this atom-level list — the
    two items no longer share a single blocker:
    - `ReduceStereocenterDensity`'s `target_atoms` — used to always be an
      empty `Vec` (see "Simplification suggestions" below); **implemented
      round 22 part 6** — `stereo_centers` gives real atom indices
      directly, no new design question, no scoring change.
    - Quaternary-carbon adjacency (`ReduceAdjacentQuaternaryCenters`) is
      *not* just a wiring task even now: `stereo_centers` gives candidate
      atoms, but "adjacent quaternary centers" as a burden signal still
      needs a new detection rule (what counts as adjacent), very likely a
      new `FindingCode`, and a new weighted contribution — i.e. a scoring
      -formula change requiring the same root-cause-first discipline as
      the round 22 external-benchmark findings above, not a documentation
      -availability question anymore. See `ROADMAP.md`'s "Needs scoping"
      section (gitignored).
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
