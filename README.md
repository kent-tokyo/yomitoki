# rensei

Fast, explainable, route-free molecular synthesizability diagnostics.

RENSEI is a fast, explainable, route-free molecular
synthesizability diagnostics library built on [chematic](https://github.com/kent-tokyo/chematic).

Instead of returning only a synthetic accessibility score,
RENSEI reports why a molecule appears accessible or difficult,
how confident that assessment is, and which structural factors
dominate the result.

> **Status: v0.1 in progress.** Only the `input_quality`/`applicability` and
> `ring_topology` components are implemented so far. See
> [`docs/architecture.md`](docs/architecture.md) for the current scope and
> what's still missing.

## What it does

* Parses a molecule (via `chematic`) and returns a structured
  `SynthesizabilityReport`, not a single number.
* Breaks the assessment down into independent components (ring topology,
  input quality/applicability today; size/topology, stereochemical burden,
  fragment rarity, and functional-group liabilities are planned).
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

```text
Verdict: Challenging
Synthesizability: 0.41
Confidence: 0.86

Findings:
- RING_BRIDGED_COMPLEXITY: bridged bicyclic ring system
```

Every report also carries a `Provenance` block (schema version, rensei
version, chematic version, ruleset version, config hash) so results are
comparable across versions — see §16 of the design spec (`AGENTS.md`) and
`docs/architecture.md`.

## Component status (v0.1)

| Component | Status |
|---|---|
| `input_quality` / applicability | implemented |
| `ring_topology` | implemented |
| `size_topology` | not yet implemented |
| `stereochemical_burden` | not yet implemented |
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

* v0.1 only implements two of the six planned components (see table above);
  `overall.difficulty`/`overall.synthesizability` currently reflect ring
  topology burden alone.
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

See `AGENTS.md` (development spec) for the full phased roadmap: size/topology
and stereochemical-burden components, fragment rarity, functional-group
liabilities, calibration against SAscore/RAscore/route-search outcomes, a
CLI, and eventually Python bindings.
