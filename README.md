# yomitoki

[![CI](https://github.com/kent-tokyo/yomitoki/actions/workflows/ci.yml/badge.svg)](https://github.com/kent-tokyo/yomitoki/actions/workflows/ci.yml)
[![crates.io](https://img.shields.io/crates/v/yomitoki.svg)](https://crates.io/crates/yomitoki)
[![docs.rs](https://docs.rs/yomitoki/badge.svg)](https://docs.rs/yomitoki)
[![License](https://img.shields.io/crates/l/yomitoki.svg)](#license)

**English** | [日本語](README_ja.md) | [中文](README_zh.md)

Fast, explainable, route-free molecular synthesizability diagnostics.

yomitoki is a fast, explainable, route-free molecular
synthesizability diagnostics library built on [chematic](https://github.com/kent-tokyo/chematic).

Instead of returning only a synthetic accessibility score,
yomitoki reads molecular structure and explains why a molecule
appears easy or difficult to synthesize. It identifies the
structural evidence behind the assessment and reports how
confident that judgment is.

The name comes from the Japanese word *yomitoki*（読み解き）,
meaning to carefully examine something and uncover its meaning —
that is the library's whole job: not to change the molecule, but
to read it and explain what it finds.

> yomitoki does not merely estimate synthesizability; it exposes
> the evidence and reasoning behind the estimate.

> **Status: `0.1.1` — correctness/dependency/infrastructure patch on
> `0.1.0`, the first non-alpha release.**
> All six planned components are implemented, including
> `fragment_precedent` (renamed from `fragment_rarity` in round 18, since
> it argues difficulty both up *and* down, not just up) — opt-in: no
> fragment-frequency corpus ships with yomitoki itself (AGENTS.md §5.4
> forbids embedding one directly as a huge binary), so it stays inactive
> unless you build one (`tools/build-fragment-corpus/`) and configure it
> (`AnalysisConfig.fragment_model`). **`fragment_precedent` is an
> explanatory reference-corpus signal, not a direct synthetic-difficulty
> term** — round 20 found the corpus-relative signal too corpus-sensitive
> to trust as a scoring input (two honestly-labeled synthesis-focused
> corpora disagreed with each other on its direction 34.6% of the time
> over 500 probe molecules; plain pyridine swung between
> `LikelyAccessible` and `HighlyChallenging` purely from which corpus was
> configured), so round 21 removed it from `overall.difficulty` entirely
> (option C). **If you configure a corpus:** `fragment_precedent` is still
> computed and still reported (`SynthesizabilityReport.fragment_precedent`)
> as explanatory evidence, but it can no longer change
> `overall.difficulty`, `dominant_penalties`, or `dominant_supports` —
> configuring a corpus (or switching which one) never changes the score,
> only the evidence available alongside it. See Limitations for the full
> before/after data and `rules.rs`'s "Fragment precedent" section for the
> complete round 16–21 history. See [`CHANGELOG.md`](CHANGELOG.md) for the
> full version history.
> See [`docs/architecture.md`](docs/architecture.md) for the current scope
> and what's still missing.

## Where it sits

```text
chematic    Molecular representation and cheminformatics
    |
yomitoki    Read and explain molecular synthesizability
    |
renkin      Plan retrosynthetic routes
```

[chematic](https://github.com/kent-tokyo/chematic) · [renkin](https://github.com/kent-tokyo/renkin)

yomitoki never runs route search — that boundary is permanent, not a v0.1
scoping choice. See "What it does not do" below.

## What it does

* Parses a molecule (via `chematic`) and returns a structured
  `SynthesizabilityReport`, not a single number.
* Breaks the assessment down into independent components (ring topology,
  size/topology, stereochemical burden, functional-group liabilities, input
  quality/applicability, and fragment rarity — the last is opt-in, see
  "Limitations" below).
* Separates **score** (synthesizability/difficulty), **confidence** (how
  reliable the judgment is), and **applicability** (whether the molecule is
  even in the model's domain) into distinct fields — a hard-to-make molecule
  is not automatically a low-confidence one.
* Emits machine-readable finding codes with structured evidence, not just
  prose.
* Never runs retrosynthesis search. yomitoki evaluates a molecule on its own;
  it does not plan a route to make it.
* Ships a `yomitoki` CLI for single-molecule and batch (`.sdf`/SMILES-file)
  analysis — see Command-line interface below.
* `analyze_batch(&[Molecule], &AnalysisConfig) -> Vec<Result<...>>` gives
  library callers the same input-order-preserving batch entry point, without
  going through the CLI or a file format.

## What it does not do

* Retrosynthesis planning, reaction template application, precursor
  generation, or route ranking — that's [RENKIN](https://github.com/kent-tokyo/renkin)'s job.
* Molecule parsing, ring perception, aromaticity, or stereochemistry
  assignment — that's [chematic](https://github.com/kent-tokyo/chematic)'s job; yomitoki only consumes it.
* Toxicity, SDS/hazard classification, yield prediction, or cost prediction.
* Full periodic-table or organometallic/polymer coverage in v0.1.

## Quick start

```rust
use yomitoki::{analyze_smiles, AnalysisConfig};

let config = AnalysisConfig::default();
let report = analyze_smiles("C1CC2CCC1C2", &config)?; // norbornane

println!("{:?}", report.overall.verdict);
println!("difficulty = {}", report.overall.difficulty.value());
println!("confidence = {}", report.overall.confidence.value());

for finding in &report.findings {
    println!("{:?}: {}", finding.code, finding.explanation);
}
```

Run the full example:

```bash
cargo run --example basic
```

## Command-line interface

```bash
yomitoki analyze "C1CC2CCC1C2" --format json
yomitoki analyze --input molecules.sdf --format jsonl --output reports.jsonl
```

* `yomitoki analyze "<SMILES>" [--format human|json|jsonl] [--fragment-corpus <dir>]`
  — analyze one molecule from an argument.
* `yomitoki analyze --input <file> [--format human|json|jsonl] [--output <file>] [--fragment-corpus <dir>]`
  — batch mode. `<file>` may be a `.sdf` file or a SMILES-per-line file
  (optionally with a whitespace-separated name column, the standard `.smi`
  convention).
* Batch mode preserves input order and never stops on one record's failure —
  a failed record becomes an error entry (JSON `"error"` field, or an
  `ERROR:` block in human format), not a skipped one. The process exits
  non-zero only after every record has been attempted, if any failed.
* `jsonl` output uses the same `{"input", "report"|"error"}` wrapper shape
  in both single-molecule and batch mode, so a downstream line-by-line
  parser sees one schema regardless of which invocation form produced it.
* Exit codes: `0` success, `1` a molecule failed to parse/analyze (single
  mode) or at least one batch record failed, `2` a usage error (bad
  arguments).
* `--fragment-corpus <dir>` loads a `tools/build-fragment-corpus` output
  directory and enables `fragment_precedent` for the run (loaded once,
  before any molecule is analyzed). Without it, reports have
  `fragment_precedent: null`, same as before this flag existed — no corpus
  ships with yomitoki itself, see Limitations below.

## Report shape

Actual output of `cargo run --example basic`, current as of this component
set. Norbornane (`C1CC2CCC1C2`) exercises ring topology, and its bridged
ring produces a simplification suggestion:

```text
Verdict: ModeratelyAccessible
Synthesizability: 0.66
Confidence: 1.00
Dominant penalties:
1. Bridged ring system spanning 7 atoms — bridgehead connectivity typically increases synthetic difficulty.
Simplification suggestions (heuristic, not a guarantee):
1. ReplaceBridgedRingWithMonocyclicAnalog: Bridgehead connectivity in this ring system is a direct driver of the ring_topology contribution to difficulty. A monocyclic (or less-fused) analog, if the target application allows one, would remove this specific burden — this is a structural heuristic, not a guarantee the replacement is chemically equivalent or that synthesis actually becomes easier.
```

A stereocenter-dense fragment (`CC(O)C(N)C(C)C(O)C(N)C`) exercises
`stereochemical_burden` and `functional_group_liability` (difficulty) plus
applicability's independent confidence penalty for unspecified
stereochemistry — note that difficulty and confidence move separately, not
together:

```text
Verdict: ModeratelyAccessible
Synthesizability: 0.69
Confidence: 0.85
Dominant penalties:
1. 5 tetrahedral stereocenter(s) (specified or unspecified) requiring synthetic control.
2. Stereocenter density 0.42 is above the 0.25 threshold — stereocenters are concentrated in a compact region, leaving little room for staged, orthogonal control.
3. Reactive/unstable functional group detected: primary amine (Brenk et al. 2008 structural alert).
Simplification suggestions (heuristic, not a guarantee):
1. ReduceStereocenterDensity: Stereocenters are concentrated in a compact region, leaving little room for staged, orthogonal stereocontrol. Reducing the number of stereocenters, or spreading them further apart in the structure, would lower this contribution to difficulty — this is a structural heuristic, not a guarantee.
```

An epoxide (`C1CO1`) exercises `functional_group_liability` on its own —
it wraps chematic's Brenk et al. (2008) structural-alert set directly:

```text
Verdict: LikelyAccessible
Synthesizability: 0.87
Confidence: 1.00
Dominant penalties:
1. Reactive/unstable functional group detected: epoxide (Brenk et al. 2008 structural alert).
```

A 9-membered ring (`C1CCCCCCCC1`) exercises the macrocycle branch of
`ring_topology`, and its own simplification suggestion:

```text
Verdict: LikelyAccessible
Synthesizability: 0.75
Confidence: 1.00
Dominant penalties:
1. Macrocyclic ring of 9 atoms (at or above the 9-atom macrocycle threshold).
Simplification suggestions (heuristic, not a guarantee):
1. SimplifyMacrocyclicClosure: Macrocyclic ring closure is a direct driver of the ring_topology contribution to difficulty (large-ring closures often need high-dilution or specialized macrocyclization methods). A smaller ring or acyclic analog, if chemically acceptable, would remove this burden — this is a structural heuristic, not a guarantee.
```

Pentaerythritol tetraacetate (`CC(=O)OCC(COC(C)=O)(COC(C)=O)COC(C)=O`) has 4
distinct, non-adjacent ester environments and exercises
`functional_group_liability`'s "dense functionalization" signal
(`chematic::chem::identify_functional_groups`, Ertl 2017 clustering) on top
of its Brenk alerts:

```text
Verdict: LikelyAccessible
Synthesizability: 0.81
Confidence: 1.00
Dominant penalties:
1. 4 distinct functional-group environments (Ertl 2017 clustering), above the 3 threshold — multiple independent reactive/functional regions can compete for reagent selectivity and complicate protecting-group strategy.
2. Reactive/unstable functional group detected: ketone alpha (Brenk et al. 2008 structural alert).
3. Reactive/unstable functional group detected: acetal ketal (Brenk et al. 2008 structural alert).
```

Alaninate (`C[C@@H](N)C(=O)[O-]`, deprotonated alanine) has a specified
stereocenter *and* a negatively charged atom. Through chematic 0.12,
the negative charge triggered a real upstream overflow bug
([chematic#267](https://github.com/kent-tokyo/chematic/issues/267)) —
yomitoki worked around it by skipping stereo analysis entirely for any
negatively charged atom, at the cost of a real confidence penalty and a
silently-zeroed `stereochemical_burden` for every such molecule.
**chematic 0.13.0 fixed the bug directly** (verified: alaninate now
returns the identical result its neutral-acid form would); the
workaround was removed accordingly, and this molecule now gets full,
correct stereo analysis like any other:

```text
Verdict: LikelyAccessible
Synthesizability: 0.86
Confidence: 1.00
Dominant penalties:
1. 1 tetrahedral stereocenter(s) (specified or unspecified) requiring synthetic control.
2. Reactive/unstable functional group detected: primary amine (Brenk et al. 2008 structural alert).
3. Reactive/unstable functional group detected: acetal ketal (Brenk et al. 2008 structural alert).
```

Configuring a fragment corpus (see below) only changes the `fragment_precedent`
evidence field's contents — since round 21 (option C), `overall.difficulty`/
`overall.synthesizability` are computed from `ring_topology`/`size_topology`/
`stereochemical_burden`/`functional_group_liability` alone and are
identical with or without a corpus configured.

Every report also carries a `Provenance` block (schema version, yomitoki
version, chematic version, ruleset version, fragment-corpus model version,
config hash) so results are comparable across versions — see
`docs/architecture.md`.

## Component status (v0.1)

| Component | Status |
|---|---|
| `input_quality` / applicability | implemented |
| `ring_topology` | implemented |
| `size_topology` | implemented |
| `stereochemical_burden` | implemented (tetrahedral centers only — see Limitations) |
| `functional_group_liability` | implemented (reactive/unstable groups + dense functionalization — see Limitations) |

`fragment_precedent` is implemented and opt-in (`None` unless
`AnalysisConfig.fragment_model` has a corpus configured), but is **not** a
`ComponentScores` field — since round 21 it doesn't contribute to
`overall.difficulty`, so it lives in its own top-level
`SynthesizabilityReport.fragment_precedent: Option<FragmentPrecedentEvidence>`
field: a corpus-relative percentile signal reported as explanatory
evidence only, see Limitations.

Components stay `None` in `ComponentScores`/`fragment_precedent` when
genuinely not evaluated, never as fabricated zero scores.

`suggestions: Vec<SimplificationSuggestion>` is populated for 3 of its 6
possible codes (`ReplaceBridgedRingWithMonocyclicAnalog`,
`SimplifyMacrocyclicClosure`, `ReduceStereocenterDensity`) — see
Limitations. `IncreaseFragmentPrecedent` is retired (unreachable since
round 21 — kept in the enum for schema stability, never emitted, since
"this would lower this contribution to difficulty" is no longer a true
statement). Every suggestion is diagnostic-only, heuristic, and never
claims certainty (`expected_effect` is always `MayReduceDifficulty`, never
`LikelyReducesDifficulty`).

## How this differs from existing tools

* **SAscore** returns fragment-frequency and complexity penalties as a
  single number. yomitoki returns component-wise diagnostics, confidence,
  applicability, evidence, and simplification suggestions.
* **SYBA** is an easy/hard classifier. yomitoki is a diagnostic and
  explanation tool.
* **SCScore** is a learned synthetic-complexity score. yomitoki decomposes
  transparent, chemically-named factors instead.
* **RAscore** approximates retrosynthesis success. yomitoki is route-free and
  explains the structural reasons behind its assessment.
* **BR-SAScore** retrains SAscore's fragment table on USPTO
  reaction/eMolecules building-block data and reports a single
  reaction/building-block-informed score. yomitoki returns five
  independent structural components plus applicability, confidence, and
  machine-readable findings/simplification-suggestion codes — a
  structured diagnostic report, not a single number, and requires no
  reaction corpus to produce `overall.difficulty` at all (corpus choice
  is contractually guaranteed not to change it — see
  [External benchmark](#external-benchmark-v010) below for how the two
  actually compare on accuracy).
* **AiZynthFinder, ASKCOS, RENKIN** are route planners. yomitoki never
  generates a route.

## Comparison with SAscore

A minimum in-process comparison against `chematic::chem::sa_score` (Ertl &
Schuffenhauer 2009) — the completion criterion in AGENTS.md §27. Not a
calibration or accuracy claim: the two scores aren't fit against each other
and measure different things (SAscore: fragment frequency + complexity
penalty; yomitoki: structural burden, decomposed by component). Scales run
in opposite directions and aren't rescaled onto a shared axis here — SAscore
is `1` (easy) to `10` (hard); yomitoki's `difficulty` is `0.0` to `1.0`.

Real output of `cargo run --example sa_score_comparison`:

```text
molecule                                sa_score      yomitoki_diff  verdict
ethanol                                     3.45               0.01  LikelyAccessible
benzene                                     2.40               0.10  LikelyAccessible
norbornane (bridged)                        8.20               0.34  ModeratelyAccessible
stereocenter-dense fragment                 8.32               0.31  ModeratelyAccessible
epoxide                                     6.23               0.13  LikelyAccessible
aspirin                                     4.67               0.27  ModeratelyAccessible
paracetamol                                 4.56               0.24  LikelyAccessible
caffeine (fused heterocycle)                4.94               0.29  ModeratelyAccessible
acyl halide                                 6.62               0.09  LikelyAccessible
cyclopropane (strained)                     3.94               0.13  LikelyAccessible
nitrile                                     4.97               0.04  LikelyAccessible
alanine (specified stereocenter)            3.55               0.14  LikelyAccessible
bridged ring + several stereocenters       10.00               0.72  Challenging
spiro ring system                           5.52               0.20  LikelyAccessible
```

The interesting rows are where the two diverge, not where they agree — a
divergence isn't automatically a yomitoki bug. Acyl halide is the sharpest
case: SAscore rates it `6.62` (fragment-uncommon), yomitoki rates it `0.09`
(`LikelyAccessible`) — a small, cheap, extremely common acylating reagent
with essentially no structural burden by yomitoki's own model. Aspirin
(`4.67` vs. `0.27`) is the same shape as the Brenk-validity gap already
described in Limitations. Where they broadly agree (caffeine, spiro,
bridged-plus-stereo), that's not evidence either one is "correct" — neither
has been validated against real synthesis outcomes yet.

`yomitoki_diff` for "stereocenter-dense fragment" and "bridged ring +
several stereocenters" moved (`0.27→0.31`, `0.69→0.72`) with the
chematic 0.13.0 upgrade — a separate stereocenter-*counting* bug fix
from the negatively-charged-atom fix below (an implicit-hydrogen rank
-0 sentinel could collide with a real atom's normalized rank 0,
silently undercounting stereocenters at certain positions). Both
numbers are now higher, correctly, not lower — the old counts were an
undercount, not an overcount.

## External benchmark (v0.1.0)

yomitoki v0.1.0's frozen default was measured against BR-SAScore's own
TS1/TS2/TS3 test sets, alongside SAscore and BR-SAScore itself, on the
exact same molecules. Full methodology, per-molecule results, and honest
limitations: **[docs/benchmark.md](docs/benchmark.md)**.

Stated plainly, not rounded up: yomitoki is competitive with BR-SAScore
on TS1 (ROC-AUC 0.952 vs. 0.983; balanced accuracy and MCC at yomitoki's
own threshold actually exceed SAscore's), has **no discriminative power
on TS2** (ROC-AUC 0.476 — chance level, diagnosed as a genuine structural
finding: TS2's easy/hard classes are ring/size/stereo/functional-group
homogeneous under yomitoki's model), and is weaker than both competitors
on TS3 (0.673 vs. 0.839 / 0.905). The confidence-based selective-prediction
evaluation this benchmark set out to validate as a differentiator **did
not confirm that story** — on TS1, higher-confidence predictions were
measurably *less* accurate than lower-confidence ones, traced to
`overall.confidence` acting as a proxy for dataset provenance (stereo-tag
completeness) rather than for prediction correctness. All of this is
reported in `docs/benchmark.md` because it's true, not because it's
flattering — the same document states what would need to change before
these numbers improve, and yomitoki's per-round development process
treats these results as confirmatory, not as something to retune against.

**TS2's chance-level result was checked against a second, unrelated
dataset — and it reproduced.** [`benchmarks/synthesizability/DEVELOPMENT_SET.md`](benchmarks/synthesizability/DEVELOPMENT_SET.md)
validates the frozen baseline against MPScore, a published
expert-chemist-labeled dataset (three chemists' independent
easy/difficult ratings, methodologically unrelated to TS1/2/3's
retrosynthesis-planner-derived labels, ~0.03% molecule overlap):
ROC-AUC 0.513 on the full set, still inside the 95% CI for chance. Two
independent ground-truth sources — one algorithmic, one human — now
agree that yomitoki's four structural components miss a large share of
what makes a molecule hard to synthesize in practice (72.6% false
negatives on that set). That document also runs a controlled,
unlabeled ablation panel isolating *which* structural axes the
components respond to (and where they saturate or respond backwards),
and records four evidence-based design-change candidates without
implementing any of them yet — development-only, kept deliberately
separate from the TS1/2/3 confirmatory numbers above.

## Limitations

* All six planned components are implemented (see table above), but
  `fragment_precedent` **never** contributes to `overall.difficulty`/
  `overall.synthesizability` — since round 21 (option C), configuring a
  corpus (`AnalysisConfig.fragment_model`; no corpus ships with yomitoki
  itself, AGENTS.md §5.4) changes what `fragment_precedent` reports, not
  what `overall.difficulty` measures. `overall.difficulty` always reflects
  exactly `ring_topology`/`size_topology`/`stereochemical_burden`/
  `functional_group_liability`, corpus configured or not.
* `size_topology`'s rotatable-bond term over-penalizes simple, commercially
  available long unbranched chains (many rotatable bonds, essentially no
  synthetic difficulty) — e.g. dodecane's `overall.difficulty` is `0.068`,
  aspirin's (tripping several Brenk alerts in `functional_group_liability`)
  is `0.273`, `ModeratelyAccessible`, despite being one of the most
  trivially synthesizable molecules there is. Through round 20,
  `fragment_precedent` corrected this once a corpus was configured
  (dodecane `→ 0.000`, aspirin `→ 0.095`); **round 21 removed that
  correction** (see the next point for why), so this over-penalization is
  now an acknowledged, currently-unaddressed limitation regardless of
  corpus configuration. `fragment_precedent` still reports (as explanatory
  evidence, not a score adjustment) that such fragments are strongly
  precedented, so a report reader can see the mismatch even though the
  score no longer reflects it.
* **`fragment_precedent` is an explanatory reference-corpus signal, not a
  direct synthetic-difficulty term** — this is the public contract, not
  just a caveat. It used to feed `overall.difficulty` (rounds 17–20); round
  20's cross-corpus validation found that unsafe: configured against
  ChEMBL vs. two different real, honestly-labeled synthesis-focused
  corpora (Open Reaction Database, SynRXN), several structurally
  -legitimate molecules (caffeine, norbornane, spiro/stereocenter-dense
  systems) scored *harder* under one corpus and *not* under another, with
  no way to predict which in advance — worst case, plain pyridine (no
  plausible synthetic-difficulty story at all) swung between
  `LikelyAccessible` and `HighlyChallenging` purely from which corpus was
  configured, driven entirely by this component's own uncapped penalty
  term. **Round 21 (option C) removed `fragment_precedent` from
  `overall.difficulty` entirely** — the signal is still computed exactly
  the same way and still fully reported
  (`SynthesizabilityReport.fragment_precedent`), but it can no longer
  change a score, a verdict, `dominant_penalties`, or `dominant_supports`.
  Verified end-to-end: `overall.difficulty` is now bit-for-bit identical
  regardless of which corpus (ChEMBL/ORD/SynRXN/none) is configured, for
  every molecule tested. See `rules.rs`'s "Fragment precedent" section for
  the full round 16–21 history and reasoning.
* `stereochemical_burden` only covers tetrahedral stereocenter count and
  density. Investigated and still not implemented, each for a different
  reason (see `docs/architecture.md` for the full evidence):
  * E/Z double-bond stereo — chematic *can* assign E/Z directly from SMILES
    `/`/`\` bond markers (no 2D coordinates needed, correcting an earlier,
    inaccurate note here), but only for bonds the input SMILES already
    marked. There's no detector for a stereogenic-but-*unspecified* double
    bond the way tetrahedral centers have one, so a specified-only count
    would measure how carefully the SMILES was written, not how many E/Z
    centers exist — the same class of problem atropisomerism was rejected
    for below, arriving a different way.
  * Atropisomerism — chematic's `detect_atropisomers` was tested directly
    and disqualified: it rates unsubstituted biphenyl as an atropisomer
    when written `c1ccccc1-c2ccccc2` but not when written
    `c1ccccc1c2ccccc2` (same molecule), and rates *para*-substituted
    biphenyl identically to genuinely hindered *ortho*-substituted biphenyl.
    Wrapping it would violate yomitoki's own atom-order/representation
    invariance guarantee.
  * Contiguous stereocenter runs, quaternary-carbon adjacency — both need
    an atom-level list of stereocenter candidates (specified *and*
    unspecified), which chematic doesn't expose (only aggregate counts).
    Building that inside yomitoki would make this crate the owner of
    stereocenter perception instead of a consumer of a validated chematic
    primitive — a line every implemented component has stayed on the other
    side of so far.
  * Meso compound detection — needs graph automorphism / topological
    symmetry classes; chematic has this internally
    (`chematic-smiles::canonical_automorphism`) but doesn't expose it.
* `functional_group_liability` covers reactive/unstable functional groups
  (chematic's Brenk et al. 2008 structural-alert set directly) and dense
  functionalization (distinct functional-group cluster count, via
  chematic's Ertl 2017 `identify_functional_groups`). Mutually incompatible
  functional-group combinations and protecting-group pressure are not
  implemented — unlike the two liabilities above, neither has a citable,
  validated primitive to build on (chematic exposes none), and hand-curating
  either would be exactly the "chemically weak rules, over-generalized"
  AGENTS.md warns against. Chemoselectivity burden, polyfunctional symmetry
  breaking, and difficult oxidation-state combinations are also not
  implemented — chematic exposes no oxidation-state API to build the last
  one on. Brenk's set was validated as a med-chem screening-library
  desirability filter, not a synthetic-difficulty signal, so several of its
  alerts fire on common, cheaply-precedented groups — aspirin, for example,
  trips four Brenk alerts and lands at `ModeratelyAccessible` despite being
  trivially synthesizable, the same score regardless of corpus
  configuration since round 21. This is a known gap with the same shape as
  the rotatable-bond one above — through round 20, `fragment_precedent`
  corrected it once a corpus was configured (aspirin `0.273 → 0.095`);
  round 21 removed that correction (see the fragment_precedent contract
  point above) rather than continue tuning a signal round 20 found too
  corpus-sensitive to trust for scoring. Dense functionalization has
  its own known gap: it counts
  topologically *disconnected* functional-group clusters, so a single
  densely interconnected polyfunctional system (e.g. glucose's ring of
  hydroxyls, or a fused β-lactam) collapses to one cluster — identical in
  count to a molecule with a single, ordinary functional group.
* No fragment corpus ships with yomitoki (AGENTS.md §5.4), so out of the
  box, novel/rare substructures are not detected — `fragment_precedent` stays
  `None` until you build one (`tools/build-fragment-corpus`) and configure
  it. No decision has been made yet about shipping one by default (the
  `yomitoki-core`/`yomitoki-models`/`yomitoki-data` split §5.4 sketches, or
  a feature-flagged external file).
* Simplification suggestions cover 3 of `SuggestionCode`'s 6 variants
  (bridged ring, macrocycle, stereocenter density) — the other 3 are
  unreachable, each for a different reason: `IncreaseFragmentPrecedent`
  was retired in round 21 (it can no longer be truthfully described as
  reducing difficulty); quaternary-carbon adjacency isn't computed
  anywhere; and `brenk_matches_detailed` unions atoms per pattern rather
  than per occurrence (so "remove one of several similar reactive groups"
  can't identify which occurrence to point at). Every suggestion's
  confidence is a flat, named constant (0.5), not per-suggestion-code,
  since none are calibrated against real synthesis outcomes.
* `ApplicabilityReport.domain_distance` is always `None` until a calibration
  corpus exists (Phase 2+).
* Coverage is limited to a curated organic-element subset — no attempt at
  full periodic-table or organometallic support.
* Scores and thresholds are rule-based, not fit to any labeled dataset.
  An external benchmark now exists (see
  [External benchmark](#external-benchmark-v010) above) with mixed
  results — competitive with BR-SAScore on TS1, chance-level on TS2,
  weaker than both competitors on TS3 — and no weight/threshold has been
  changed in response to it (see that section's test-set-integrity rule).

## Reproducibility

Given the same input, `AnalysisConfig`, and yomitoki/chematic/ruleset
versions, `analyze`/`analyze_smiles` always return the same report — no
randomness is used in the core evaluation path.

## License

Licensed under either of [Apache License, Version 2.0](LICENSE-APACHE) or
[MIT license](LICENSE-MIT) at your option.

## Citation

No paper or citable release exists yet.

## Roadmap

Remaining planned work: a shipped fragment corpus (`fragment_precedent`
itself is implemented but opt-in — see Limitations), *calibration* against
SAscore/RAscore/route-search outcomes (a minimum *comparison* against
SAscore, not calibration, now exists — see above), and eventually Python
bindings.
