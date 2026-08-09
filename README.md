# rensei

Fast, explainable, route-free molecular synthesizability diagnostics.

RENSEI is a fast, explainable, route-free molecular
synthesizability diagnostics library built on [chematic](https://github.com/kent-tokyo/chematic).

Instead of returning only a synthetic accessibility score,
RENSEI reports why a molecule appears accessible or difficult,
how confident that assessment is, and which structural factors
dominate the result.

> **Status: v0.1 in progress.** Only the `input_quality`/`applicability`,
> `ring_topology`, `size_topology`, and `stereochemical_burden` components
> are implemented so far. See [`docs/architecture.md`](docs/architecture.md)
> for the current scope and what's still missing.

## What it does

* Parses a molecule (via `chematic`) and returns a structured
  `SynthesizabilityReport`, not a single number.
* Breaks the assessment down into independent components (ring topology,
  size/topology, stereochemical burden, input quality/applicability today;
  fragment rarity and functional-group liabilities are planned).
* Separates **score** (synthesizability/difficulty), **confidence** (how
  reliable the judgment is), and **applicability** (whether the molecule is
  even in the model's domain) into distinct fields — a hard-to-make molecule
  is not automatically a low-confidence one.
* Emits machine-readable finding codes with structured evidence, not just
  prose.
* Never runs retrosynthesis search. RENSEI evaluates a molecule on its own;
  it does not plan a route to make it.

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

A stereocenter-dense fragment (`CC(O)C(N)C(C)C(O)C(N)C`) exercises both
`stereochemical_burden` (difficulty) and applicability's independent
confidence penalty for unspecified stereochemistry — note that difficulty
and confidence move separately, not together:

```text
Verdict: LikelyAccessible
Synthesizability: 0.78
Confidence: 0.85
Dominant penalties:
1. 4 tetrahedral stereocenter(s) (specified or unspecified) requiring synthetic control.
2. Stereocenter density 0.33 is above the 0.25 threshold — stereocenters are concentrated in a compact region, leaving little room for staged, orthogonal control.
```

With fragment rarity and functional-group liabilities still missing,
scores overall remain lower than a full v0.1 would produce.

Every report also carries a `Provenance` block (schema version, rensei
version, chematic version, ruleset version, config hash) so results are
comparable across versions — see §16 of the design spec (`AGENTS.md`) and
`docs/architecture.md`.

## Component status (v0.1)

| Component | Status |
|---|---|
| `input_quality` / applicability | implemented |
| `ring_topology` | implemented |
| `size_topology` | implemented |
| `stereochemical_burden` | implemented (tetrahedral centers only — see Limitations) |
| `fragment_rarity` | not yet implemented |
| `functional_group_liability` | not yet implemented |

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

* v0.1 only implements four of the six planned components (see table
  above); `overall.difficulty`/`overall.synthesizability` currently reflect
  ring topology, size/topology, and stereochemical burden only.
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

See `AGENTS.md` (development spec) for the full phased roadmap:
fragment rarity, functional-group-liability component, calibration against
SAscore/RAscore/route-search outcomes, a CLI, and eventually Python
bindings.
