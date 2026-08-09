# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- Initial project scaffold.
- `SynthesizabilityReport` schema (`report.rs`), `AnalysisConfig` (`config.rs`),
  `YomitokiError` (`error.rs`).
- `input_quality`/applicability component.
- `ring_topology` burden component.
- `size_topology` burden component (molecular weight, rotatable bond count).
- `stereochemical_burden` component (tetrahedral stereocenter count and
  density).
- `functional_group_liability` component (reactive/unstable functional
  groups, via chematic's Brenk et al. 2008 structural-alert set).
- `functional_group_liability` gained "dense functionalization": distinct
  functional-group cluster count via chematic's `identify_functional_groups`
  (Ertl 2017), new `FindingCode::FunctionalGroupDense`. The first cluster is
  free; burden starts at a second, topologically disconnected functional
  region (known gap: fused/interconnected polyfunctional systems, e.g.
  glucose or penicillin V, collapse to a single cluster and don't register).
  `ruleset_version` bumped to 0.6.0.
- `examples/sa_score_comparison.rs`: minimum in-process comparison against
  `chematic::chem::sa_score` (Ertl & Schuffenhauer 2009), satisfying the
  AGENTS.md §27 v0.1 completion criterion. Not a calibration/accuracy
  claim — see `docs/architecture.md`'s "Comparison with SAscore" section.
  No `ruleset_version` bump: no scoring thresholds or weights changed.
- `analyze` / `analyze_smiles` entry points.
- CI workflow (`.github/workflows/ci.yml`): fmt/clippy/doc, `cargo test`
  on Linux and macOS, MSRV (1.88) check, `cargo-deny` license/advisory
  audit.
- Property-based tests (`tests/property_based.rs`, `proptest` dev-dependency):
  no panics, no NaN/Infinity, all scores stay in `0.0..=1.0`, finding atom
  indices stay in range, across randomized molecules and configs.
- `yomitoki` CLI binary (`src/bin/yomitoki.rs`): `yomitoki analyze "<SMILES>"
  [--format human|json|jsonl]` for a single molecule, and
  `yomitoki analyze --input <file> [--format human|json|jsonl] [--output <file>]`
  for batch analysis of a `.sdf` file or a SMILES-per-line file. Batch mode
  preserves input order and never stops on one record's failure. `jsonl`
  output uses the same `{"input", "report"|"error"}` wrapper shape in both
  single-molecule and batch mode.
- Simplification suggestions (`suggestions.rs`): `SynthesizabilityReport
  .suggestions` is now populated for 3 of `SuggestionCode`'s 6 variants
  (`ReplaceBridgedRingWithMonocyclicAnalog`, `SimplifyMacrocyclicClosure`,
  `ReduceStereocenterDensity`), derived from existing findings. Every
  suggestion is diagnostic-only and heuristic (`expected_effect` is always
  `MayReduceDifficulty`, confidence a flat named constant); the remaining 3
  codes have no underlying signal yet. `ruleset_version` bumped to 0.5.0.

### Changed

- **Renamed the project from RENSEI to YOMITOKI.** Renamed the Rust crate,
  binary, and CLI command from `rensei` to `yomitoki`; renamed `RenseiError`
  to `YomitokiError` and `Provenance.rensei_version` to
  `Provenance.yomitoki_version` (`schema_version` bumped 0.1.0 → 0.2.0 to
  reflect the field rename). Every other public type name was already
  generic (`SynthesizabilityReport`, `AnalysisConfig`, `Finding`, ...) and
  is unchanged. The project was never published to crates.io under the old
  name, so this is a clean rename with no deprecated alias. See the
  README's "Migration from RENSEI" section.
- `chematic` dependency bumped 0.11 → 0.12.
- `rust-version = "1.88"` declared explicitly in `Cargo.toml`.
- `chematic`'s `mol` feature enabled (CLI-only, for `SdfReader`/
  `SmilesRecordReader`).

### Migration

Replace:

```rust
use rensei::analyze;
```

with:

```rust
use yomitoki::analyze;
```

Replace:

```bash
rensei analyze ...
```

with:

```bash
yomitoki analyze ...
```
