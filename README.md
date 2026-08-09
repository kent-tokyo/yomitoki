# rensei

[![CI](https://github.com/kent-tokyo/rensei/actions/workflows/ci.yml/badge.svg)](https://github.com/kent-tokyo/rensei/actions/workflows/ci.yml)

Fast, explainable, route-free molecular synthesizability diagnostics.

RENSEI is a fast, explainable, route-free molecular
synthesizability diagnostics library built on [chematic](https://github.com/kent-tokyo/chematic).

Instead of returning only a synthetic accessibility score,
RENSEI reports why a molecule appears accessible or difficult,
how confident that assessment is, and which structural factors
dominate the result.

> **Status: v0.1 in progress.** Five of six planned components are
> implemented: `input_quality`/`applicability`, `ring_topology`,
> `size_topology`, `stereochemical_burden`, and
> `functional_group_liability`. Only `fragment_rarity` remains. See
> [`docs/architecture.md`](docs/architecture.md) for the current scope and
> what's still missing.

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
* Never runs retrosynthesis search. RENSEI evaluates a molecule on its own;
  it does not plan a route to make it.
* Ships a `rensei` CLI for single-molecule and batch (`.sdf`/SMILES-file)
  analysis — see Command-line interface below.

## What it does not do

* Retrosynthesis planning, reaction template application, precursor
  generation, or route ranking — that's [RENKIN](https://github.com/kent-tokyo/renkin)'s job.
* Molecule parsing, ring perception, aromaticity, or stereochemistry
  assignment — that's [chematic](https://github.com/kent-tokyo/chematic)'s job; RENSEI only consumes it.
* Toxicity, SDS/hazard classification, yield prediction, or cost prediction.
* Full periodic-table or organometallic/polymer coverage in v0.1.

## Quick start

```rust
use rensei::{analyze_smiles, AnalysisConfig};

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
rensei analyze "C1CC2CCC1C2" --format json
rensei analyze --input molecules.sdf --format jsonl --output reports.jsonl
```

* `rensei analyze "<SMILES>" [--format human|json|jsonl]` — analyze one
  molecule from an argument.
* `rensei analyze --input <file> [--format human|json|jsonl] [--output <file>]`
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
set. Norbornane (`C1CC2CCC1C2`) exercises ring topology only:

```text
Verdict: ModeratelyAccessible
Synthesizability: 0.66
Confidence: 1.00
Dominant penalties:
1. Bridged ring system spanning 7 atoms — bridgehead connectivity typically increases synthetic difficulty.
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

With fragment rarity still missing, scores overall remain lower than a
full v0.1 would produce.

Every report also carries a `Provenance` block (schema version, rensei
version, chematic version, ruleset version, config hash) so results are
comparable across versions — see `docs/architecture.md`.

## Component status (v0.1)

| Component | Status |
|---|---|
| `input_quality` / applicability | implemented |
| `ring_topology` | implemented |
| `size_topology` | implemented |
| `stereochemical_burden` | implemented (tetrahedral centers only — see Limitations) |
| `functional_group_liability` | implemented (reactive/unstable groups only — see Limitations) |
| `fragment_rarity` | not yet implemented |

Unimplemented components appear as `None` in `ComponentScores`, not as
fabricated zero scores.

## How this differs from existing tools

* **SAscore** returns fragment-frequency and complexity penalties as a
  single number. RENSEI returns component-wise diagnostics, confidence,
  applicability, evidence, and (eventually) suggestions.
* **SYBA** is an easy/hard classifier. RENSEI is a diagnostic and
  explanation tool.
* **SCScore** is a learned synthetic-complexity score. RENSEI decomposes
  transparent, chemically-named factors instead.
* **RAscore** approximates retrosynthesis success. RENSEI is route-free and
  explains the structural reasons behind its assessment.
* **AiZynthFinder, ASKCOS, RENKIN** are route planners. RENSEI never
  generates a route.

## Limitations

* v0.1 only implements five of the six planned components (see table
  above); `overall.difficulty`/`overall.synthesizability` currently reflect
  ring topology, size/topology, stereochemical burden, and functional-group
  liability only.
* `size_topology`'s rotatable-bond term over-penalizes simple, commercially
  available long unbranched chains (many rotatable bonds, essentially no
  synthetic difficulty) — this is a known gap that fragment rarity (not yet
  implemented) is meant to correct by recognizing such fragments as
  common/precedented. See `docs/architecture.md`'s "Scoring direction"
  section.
* `stereochemical_burden` only covers tetrahedral stereocenter count and
  density. E/Z double-bond stereo, atropisomerism, contiguous stereocenter
  runs, quaternary-carbon adjacency, and meso detection are not implemented
  — E/Z specifically because chematic's E/Z assignment needs 2D coordinates
  the SMILES-only pipeline doesn't have.
* `functional_group_liability` only covers reactive/unstable functional
  groups, via chematic's Brenk et al. (2008) structural-alert set directly.
  Mutually incompatible functional-group combinations, dense
  functionalization, protecting-group pressure, chemoselectivity burden,
  polyfunctional symmetry breaking, and difficult oxidation-state
  combinations are not implemented — chematic exposes no oxidation-state
  API to build the last one on. Brenk's set was validated as a med-chem
  screening-library desirability filter, not a synthetic-difficulty
  signal, so several of its alerts fire on common, cheaply-precedented
  groups — aspirin, for example, trips four Brenk alerts and lands at
  `ModeratelyAccessible` despite being trivially synthesizable. This is
  a known gap with the same shape and expected fix as the rotatable-bond
  one above (fragment rarity, once implemented).
* No fragment-rarity corpus exists yet, so novel/rare substructures are not
  detected.
* `ApplicabilityReport.domain_distance` is always `None` until a calibration
  corpus exists (Phase 2+).
* Coverage is limited to a curated organic-element subset — no attempt at
  full periodic-table or organometallic support.
* Scores and thresholds are rule-based and unvalidated against external
  benchmarks so far; no calibration or comparison results exist yet.

## Reproducibility

Given the same input, `AnalysisConfig`, and rensei/chematic/ruleset
versions, `analyze`/`analyze_smiles` always return the same report — no
randomness is used in the core evaluation path.

## License

Licensed under either of [Apache License, Version 2.0](LICENSE-APACHE) or
[MIT license](LICENSE-MIT) at your option.

## Citation

No paper or citable release exists yet.

## Roadmap

Remaining planned work: fragment rarity, calibration against
SAscore/RAscore/route-search outcomes, and eventually Python bindings.
