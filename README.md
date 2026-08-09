# yomitoki

[![CI](https://github.com/kent-tokyo/yomitoki/actions/workflows/ci.yml/badge.svg)](https://github.com/kent-tokyo/yomitoki/actions/workflows/ci.yml)

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

> **Status: v0.1 in progress.** Five of six planned components are
> implemented: `input_quality`/`applicability`, `ring_topology`,
> `size_topology`, `stereochemical_burden`, and
> `functional_group_liability`. Only `fragment_rarity` remains. See
> [`docs/architecture.md`](docs/architecture.md) for the current scope and
> what's still missing.

## Where it sits

```text
chematic    Molecular representation and cheminformatics
    |
yomitoki    Read and explain molecular synthesizability
    |
renkin      Plan retrosynthetic routes
```

yomitoki never runs route search — that boundary is permanent, not a v0.1
scoping choice. See "What it does not do" below.

## What it does

* Parses a molecule (via `chematic`) and returns a structured
  `SynthesizabilityReport`, not a single number.
* Breaks the assessment down into independent components (ring topology,
  size/topology, stereochemical burden, functional-group liabilities, input
  quality/applicability today; fragment rarity is planned).
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

* `yomitoki analyze "<SMILES>" [--format human|json|jsonl]` — analyze one
  molecule from an argument.
* `yomitoki analyze --input <file> [--format human|json|jsonl] [--output <file>]`
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
* Reports emitted by the CLI have the same `fragment_rarity: null` gap as
  every other report — see Limitations below.

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
Synthesizability: 0.74
Confidence: 0.85
Dominant penalties:
1. 4 tetrahedral stereocenter(s) (specified or unspecified) requiring synthetic control.
2. Stereocenter density 0.33 is above the 0.25 threshold — stereocenters are concentrated in a compact region, leaving little room for staged, orthogonal control.
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
stereocenter *and* a negatively charged atom — the latter is a known
chematic bug ([#267](https://github.com/kent-tokyo/chematic/issues/267)):
stereo analysis is safely skipped rather than crashing or guessing, and
confidence drops accordingly, distinctly from the ordinary "unspecified
stereocenter" case:

```text
Verdict: LikelyAccessible
Synthesizability: 0.92
Confidence: 0.60
Dominant penalties:
1. Reactive/unstable functional group detected: primary amine (Brenk et al. 2008 structural alert).
2. Reactive/unstable functional group detected: acetal ketal (Brenk et al. 2008 structural alert).
3. Stereo analysis could not be run for this molecule: it contains a negatively charged atom, which triggers an arithmetic-overflow bug in chematic's stereo perception (panics in debug builds, produces an unverified result in release builds — see chematic issue #267). Stereocenter count/density and stereo completeness are unavailable, not verified to be zero/complete.
```

With fragment rarity still missing, scores overall remain lower than a
full v0.1 would produce.

Every report also carries a `Provenance` block (schema version, yomitoki
version, chematic version, ruleset version, config hash) so results are
comparable across versions — see `docs/architecture.md`.

## Component status (v0.1)

| Component | Status |
|---|---|
| `input_quality` / applicability | implemented |
| `ring_topology` | implemented |
| `size_topology` | implemented |
| `stereochemical_burden` | implemented (tetrahedral centers only — see Limitations) |
| `functional_group_liability` | implemented (reactive/unstable groups + dense functionalization — see Limitations) |
| `fragment_rarity` | not yet implemented |

Unimplemented components appear as `None` in `ComponentScores`, not as
fabricated zero scores.

`suggestions: Vec<SimplificationSuggestion>` is populated for 3 of its 6
possible codes (`ReplaceBridgedRingWithMonocyclicAnalog`,
`SimplifyMacrocyclicClosure`, `ReduceStereocenterDensity`) — see
Limitations. Every suggestion is diagnostic-only, heuristic, and never
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
stereocenter-dense fragment                 8.32               0.27  ModeratelyAccessible
epoxide                                     6.23               0.13  LikelyAccessible
aspirin                                     4.67               0.27  ModeratelyAccessible
paracetamol                                 4.56               0.24  LikelyAccessible
caffeine (fused heterocycle)                4.94               0.29  ModeratelyAccessible
acyl halide                                 6.62               0.09  LikelyAccessible
cyclopropane (strained)                     3.94               0.13  LikelyAccessible
nitrile                                     4.97               0.04  LikelyAccessible
alanine (specified stereocenter)            3.55               0.14  LikelyAccessible
bridged ring + several stereocenters       10.00               0.69  Challenging
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

## Limitations

* v0.1 only implements five of the six planned components (see table
  above); `overall.difficulty`/`overall.synthesizability` currently reflect
  ring topology, size/topology, stereochemical burden, and functional-group
  liability only.
* Stereo analysis (both `stereo_complete` and all of `stereochemical_burden`)
  cannot run at all for a molecule containing a negatively charged atom
  (any carboxylate, sulfonate, phosphate, or other anion) — a real
  chematic bug ([#267](https://github.com/kent-tokyo/chematic/issues/267)),
  not a design choice. yomitoki never crashes or guesses on this (see
  `ApplicabilityReport.stereo_uncheckable` and the
  `StereoAnalysisSkipped` finding), but genuinely has no stereo signal for
  such molecules until it's fixed upstream.
* `size_topology`'s rotatable-bond term over-penalizes simple, commercially
  available long unbranched chains (many rotatable bonds, essentially no
  synthetic difficulty) — this is a known gap that fragment rarity (not yet
  implemented) is meant to correct by recognizing such fragments as
  common/precedented. See `docs/architecture.md`'s "Scoring direction"
  section.
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
  trivially synthesizable. This is a known gap with the same shape and
  expected fix as the rotatable-bond one above (fragment rarity, once
  implemented). Dense functionalization has its own known gap: it counts
  topologically *disconnected* functional-group clusters, so a single
  densely interconnected polyfunctional system (e.g. glucose's ring of
  hydroxyls, or a fused β-lactam) collapses to one cluster — identical in
  count to a molecule with a single, ordinary functional group.
* No fragment-rarity corpus exists yet, so novel/rare substructures are not
  detected.
* Simplification suggestions only cover 3 of `SuggestionCode`'s 6 variants
  (bridged ring, macrocycle, stereocenter density) — the other 3 need
  signals that don't exist yet: quaternary-carbon adjacency isn't computed
  anywhere, `brenk_matches_detailed` unions atoms per pattern rather than
  per occurrence (so "remove one of several similar reactive groups" can't
  identify which occurrence to point at), and "increase fragment precedent"
  needs fragment rarity, which is deferred. Every suggestion's confidence
  is a flat, named constant (0.5), not per-suggestion-code, since none are
  calibrated against real synthesis outcomes.
* `ApplicabilityReport.domain_distance` is always `None` until a calibration
  corpus exists (Phase 2+).
* Coverage is limited to a curated organic-element subset — no attempt at
  full periodic-table or organometallic support.
* Scores and thresholds are rule-based and unvalidated against external
  benchmarks so far; no calibration or comparison results exist yet.

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

Remaining planned work: fragment rarity, *calibration* against
SAscore/RAscore/route-search outcomes (a minimum *comparison* against
SAscore, not calibration, now exists — see above), and eventually Python
bindings.
