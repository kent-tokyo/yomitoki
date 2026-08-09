# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- Initial project scaffold.
- `SynthesizabilityReport` schema (`report.rs`), `AnalysisConfig` (`config.rs`),
  `RenseiError` (`error.rs`).
- `input_quality`/applicability component.
- `ring_topology` burden component.
- `size_topology` burden component (molecular weight, rotatable bond count).
- `stereochemical_burden` component (tetrahedral stereocenter count and
  density).
- `functional_group_liability` component (reactive/unstable functional
  groups, via chematic's Brenk et al. 2008 structural-alert set).
- `analyze` / `analyze_smiles` entry points.
- CI workflow (`.github/workflows/ci.yml`): fmt/clippy/doc, `cargo test`
  on Linux and macOS, MSRV (1.88) check, `cargo-deny` license/advisory
  audit.
- Property-based tests (`tests/property_based.rs`, `proptest` dev-dependency):
  no panics, no NaN/Infinity, all scores stay in `0.0..=1.0`, finding atom
  indices stay in range, across randomized molecules and configs.
- `rensei` CLI binary (`src/bin/rensei.rs`): `rensei analyze "<SMILES>"
  [--format human|json|jsonl]` for a single molecule, and
  `rensei analyze --input <file> [--format human|json|jsonl] [--output <file>]`
  for batch analysis of a `.sdf` file or a SMILES-per-line file. Batch mode
  preserves input order and never stops on one record's failure. `jsonl`
  output uses the same `{"input", "report"|"error"}` wrapper shape in both
  single-molecule and batch mode.

### Changed

- `chematic` dependency bumped 0.11 → 0.12.
- `rust-version = "1.88"` declared explicitly in `Cargo.toml`.
- `chematic`'s `mol` feature enabled (CLI-only, for `SdfReader`/
  `SmilesRecordReader`).
