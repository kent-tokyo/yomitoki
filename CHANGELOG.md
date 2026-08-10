# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- `fragment_rarity` component (AGENTS.md §5.4), implemented and opt-in.
  `ComponentScores.fragment_rarity` is `Some` only when
  `AnalysisConfig.fragment_model` has a `FragmentCorpus` configured —
  `None` by default, since no corpus ships with yomitoki itself (§5.4
  forbids embedding one directly in the library as a huge binary).
  `FragmentCorpus::load_dir` loads a corpus built by
  `tools/build-fragment-corpus` (a separate, explicitly fallible step from
  `analyze` itself — parsing stays the only fallible step inside `analyze`,
  per AGENTS.md §17). Scores from *mean* document frequency across a
  molecule's `chematic::fp::morgan_fp_counts` fragments (chosen over
  minimum; see `rules::FRAGMENT_RARITY_WEIGHT`'s doc). New
  `FindingCode::FragmentRarityHigh`, new `SuggestionCode::
  IncreaseFragmentPrecedent` derivation, new `YomitokiError::
  ModelLoadError`, new `Provenance.model_version` field. New `chematic`
  `fp` feature dependency. `AGGREGATE_WEIGHT_FRAGMENT_RARITY` and
  `FRAGMENT_RARITY_*` constants are first-pass, undocumented-as-calibrated
  values — see their doc comments in `rules.rs` for known limitations
  (`FRAGMENT_RARITY_BURDEN_SCALE`'s ceiling effect in particular).
  `ruleset_version` bumped to 0.8.0, `schema_version` to 0.3.0.
  No corpus-distribution decision has been made yet (the `yomitoki-core`/
  `yomitoki-models`/`yomitoki-data` split §5.4 sketches, vs. a
  feature-flagged external file) — build one locally with
  `tools/build-fragment-corpus` in the meantime.

## [0.1.0-alpha.1] - 2026-08-10

Public preview — a pre-release, not the completed v0.1 scope. Five of six
planned components are implemented; `fragment_rarity` remains (its
corpus-build pipeline exists, the scoring component that consumes it
doesn't). Published to reserve the crate name and let the public API,
`docs.rs` rendering, and package metadata get real feedback before a
non-alpha `0.1.0`.

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
- `analyze_batch(&[Molecule], &AnalysisConfig) -> Vec<Result<...>>`
  (AGENTS.md §18): a library-level batch entry point, input-order
  preserving, independent of the CLI's own batch mode. Sequential in v0.1
  (parallelism is optional per spec, not required); each molecule's result
  is independent of every other's, so it's safe to parallelize later
  without changing output.
- `#![warn(missing_docs)]` on the crate root, and every public item now has
  its own rustdoc (previously only container-level doc comments existed —
  97 individual items across `report.rs`/`config.rs`/`error.rs`/`analyze.rs`
  had none). Permanent, not a one-time cleanup: this lint now catches any
  future public API addition that lands without documentation, per
  AGENTS.md §29 ("public APIにはrustdocを書く").
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
- `tools/build-fragment-corpus/`: a standalone, unpublished build tool
  (not part of this package) that builds the fragment-frequency corpus
  `fragment_rarity` needs — parse/filter/dedup/fragment a molecule corpus
  into a `fragment_frequencies.json` + provenance manifest, reusing
  `chematic::mol` readers, `canonical_smiles`, and `morgan_fp_counts`
  directly. Validated against the real ChEMBL 37 chemreps file (2,897,819
  records, zero parse errors). `yomitoki::SUPPORTED_ELEMENTS` is now `pub`
  (`#[doc(hidden)]`) so the tool filters with the library's exact element
  set. The `fragment_rarity` *scoring component* itself is not yet
  implemented — this is the corpus-build pipeline only.

### Fixed

- A negatively charged atom (any carboxylate, sulfonate, phosphate, or
  other anion) previously panicked `analyze`/`analyze_smiles`/
  `analyze_batch` in debug builds, and silently risked a corrupted result
  in release builds — an arithmetic-overflow bug in chematic's stereo
  perception (`Atom.charge: i8` cast to `u64` before multiplying, which
  overflows for any negative value), filed upstream as
  [chematic#267](https://github.com/kent-tokyo/chematic/issues/267).
  `components::has_negatively_charged_atom` now guards both call sites
  (`applicability`, `stereochemical_burden`); stereo analysis is skipped
  with a new `FindingCode::StereoAnalysisSkipped` finding and a new
  `ApplicabilityReport.stereo_uncheckable` field, never a crash and never
  a fabricated "zero stereocenters" claim. New
  `CONFIDENCE_PENALTY_STEREO_UNCHECKABLE` constant; `ruleset_version`
  bumped to 0.7.0.

### Changed

- `chematic` dependency bumped 0.11 → 0.12.
- `rust-version = "1.88"` declared explicitly in `Cargo.toml`.
- `chematic`'s `mol` feature enabled (CLI-only, for `SdfReader`/
  `SmilesRecordReader`).
