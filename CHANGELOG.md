# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- `docs/benchmark.md`: first external accuracy/selective-prediction/
  throughput benchmark of the v0.1.0 frozen default against BR-SAScore's
  TS1/TS2/TS3 test sets, SAscore, and BR-SAScore. No crate code changed —
  this is measurement only. Headline, reported in full: competitive with
  BR-SAScore on TS1 (ROC-AUC 0.952 vs. 0.983), no discriminative power on
  TS2 (0.476, chance level — diagnosed as genuine structural homogeneity
  between TS2's classes, not a bug), weaker than both competitors on TS3
  (0.673 vs. 0.839/0.905). The confidence-based selective-prediction
  differentiator this benchmark was designed to validate was **not
  confirmed** — on TS1, `overall.confidence` is anti-correlated with
  prediction correctness (a dataset-provenance confound via
  `stereo_complete`), reported rather than hidden. `benchmarks/synthesizability/`
  holds the (Rust-crate-independent, Python/RDKit-based) harness; see its
  README for reproduction.
- `benchmarks/synthesizability/DEVELOPMENT_SET.md`: an unlabeled ablation
  panel diagnosing why `overall.difficulty` scored at chance level on
  TS2, plus a validation of that finding against MPScore, an independent
  expert-chemist-labeled dataset (ROC-AUC 0.513, also chance-level,
  reproducing TS2's result on a methodologically unrelated ground truth
  with ~0.03% molecule overlap). Development-only — no scoring change,
  kept explicitly separate from the confirmatory TS1/2/3 numbers.

### Fixed

- The long-standing workaround for [chematic#267](https://github.com/kent-tokyo/chematic/issues/267)
  (a negatively charged atom overflowing chematic's internal Morgan-rank
  computation, panicking in debug builds) has been **removed** — chematic
  0.13.0 fixed the bug directly, verified independently before removing
  the workaround. `applicability` and `stereochemical_burden` now run
  full stereo analysis unconditionally, including for every carboxylate,
  sulfonate, phosphate, or other anion-bearing molecule that previously
  got a hardcoded `stereo_uncheckable=true`/zero-burden fallback instead
  of a real answer. `ApplicabilityReport.stereo_uncheckable` and
  `FindingCode::StereoAnalysisSkipped` stay in the schema (never
  removed) but have no remaining trigger.
- **Behavior change, not just a bug fix**: `overall.difficulty`/
  `overall.confidence` for any molecule with a negatively charged atom
  will now differ from previous versions — previously fabricated/floored
  values are replaced with real computed ones. See `docs/architecture.md`'s
  "Negatively charged atoms" section for the full before/after story,
  including alaninate's exact numbers (confidence 0.60 → 1.00,
  synthesizability 0.92 → 0.86, the latter now correctly reflecting its
  real stereocenter instead of a fabricated zero).

### Changed

- `chematic` dependency bumped `0.12` → `0.13`. No breaking changes for
  yomitoki's actual usage (yomitoki doesn't enable the `3d`/`ff`
  features chematic 0.13's other breaking changes affect).

### Known consequence, investigated and decided (not changed)

- Removing `CONFIDENCE_PENALTY_STEREO_UNCHECKABLE` raised the achievable
  floor of `overall.confidence` from 0.3 to 0.425. `Standard`/`Strict`
  strictness's `Indeterminate` thresholds (0.45/0.6) are unaffected;
  `Lenient`'s threshold (0.3) was calibrated against the now-removed
  lower floor and is currently unreachable via applicability penalties
  alone, so `Lenient` strictness never abstains on confidence grounds in
  practice. **Investigated as a recalibration candidate and decided
  NO-GO — kept at 0.3.** `Lenient`'s only tested/documented contract is
  a relative ordering (`Lenient` <= `Standard` <= `Strict`, "most
  tolerant of the three"), which still holds; no number was ever
  promised to make it fire. With `Standard` held fixed and only four
  confidence values reachable (`{1.0, 0.85, 0.5, 0.425}`), no threshold
  value both changes today's behavior and stays distinct from
  `Standard` — anything that fires at all collapses onto the same case
  `Standard` already catches. See
  `rules::INDETERMINATE_CONFIDENCE_THRESHOLD_LENIENT`'s doc comment for
  the full reasoning.

## [0.1.0] - 2026-08-10

First non-alpha release. Closes out the `fragment_precedent` corpus
-semantics work that `0.1.0-alpha.2` was published pending (see that
section below): renamed from `fragment_rarity`, given a corpus-domain
provenance contract, stress-tested against two independent
synthesis-focused reference corpora (ORD, SynRXN) plus a 500-molecule
generated probe panel, and — because that testing found the
corpus-relative signal too corpus-sensitive to trust as a scoring input —
removed from `overall.difficulty` entirely and re-reported as independent
explanatory evidence (option C). `overall.difficulty` is now provably
corpus-invariant: verified end-to-end that it's bit-for-bit identical
across ChEMBL/ORD/SynRXN/no-corpus for every molecule in the validation
panels, while the `fragment_precedent` signal itself still genuinely
varies per corpus. See `rules.rs`'s "Fragment precedent" section for the
complete round 16–21 history and reasoning.

### Validated (update — supersedes the round-19 entry below)

- **Round-20 robustness test of round 19's cross-corpus validation —
  result: NO-GO for `v0.1.0` as currently wired, recommended contract C.**
  Round 19 concluded CONDITIONAL GO (keep `fragment_precedent` in
  `overall.difficulty`) from a 15-molecule panel against one synthesis
  -focused corpus (ORD). This round tested that conclusion against a
  second synthesis-focused corpus (SynRXN v0.0.8, USPTO-rooted like most
  of ORD — a preprocessing/curation-robustness test, not an independent
  -domain test, since the two corpora's product pools overlap 83% at the
  molecule level) and a 500-molecule *generated*, corpus-independent probe
  panel. Result: ORD and SynRXN disagree with each other on
  `fragment_precedent`'s penalty/support direction 34.6% of the time
  (Spearman rho=0.48), worse agreement than either has with ChEMBL.
  Clearest case, verified by direct fragment-level query: plain pyridine
  scores `overall.difficulty=1.0` (`HighlyChallenging`) against ORD and
  `0.095` (`LikelyAccessible`) against ChEMBL or SynRXN, with the other
  four components summing to only `0.119` — `fragment_precedent`'s own
  uncapped penalty term alone drives the result. **Recommended contract:
  C** (remove `fragment_precedent` from `overall.difficulty`, keep it as
  explanatory-only evidence) — not implemented this round (evaluation
  round; formula/weight/cap changes were out of scope), tracked as the
  actual blocker for a non-alpha `0.1.0`. No formula, weight, or cap was
  changed in response to this round's data, matching round 19's
  discipline. Full methodology in
  `tasks/upstream_and_corpus_research.md` Part 7 (gitignored); durable
  summary in `rules.rs`'s "Fragment precedent" section and
  `docs/architecture.md`'s roadmap.

### Changed

- **`fragment_precedent` removed from `overall.difficulty` (round 21,
  option C — implements round 20's recommendation).**
  `fragment_precedent` **is an explanatory reference-corpus signal, not a
  direct synthetic-difficulty term** — it no longer contributes to
  `overall.difficulty` in any way, for any configured corpus.
  **Migration from `0.1.0-alpha.2`/round-20 code:**
  - `ComponentScores` no longer has a `fragment_precedent` field (it held
    only difficulty-contributing components; `fragment_precedent` no
    longer contributes).
  - New `SynthesizabilityReport.fragment_precedent:
    Option<FragmentPrecedentEvidence>` (top-level, not nested under
    `components`) carries the same `signed_signal`/`precedent_penalty`/
    `precedent_support`/`confidence`/`findings` data as before, now with
    no cap and no aggregation role — pure evidence.
  - `dominant_penalties`/`dominant_supports` never contain a
    `fragment_precedent` entry anymore; `dominant_supports` is
    consequently always empty in v0.1 (its only source was
    `fragment_precedent`'s precedent-support case), kept in the schema
    for a future support-flavored component.
  - `SuggestionCode::IncreaseFragmentPrecedent` is retired (kept in the
    `#[non_exhaustive]` enum for schema stability, never emitted) — "this
    would lower this contribution to difficulty" is no longer a true
    statement.
  - `weak`/`strong` precedent `Finding`s (`FindingCode::
    FragmentPrecedentWeak`/`FragmentPrecedentStrong`) are unchanged and
    still produced — only their effect on scoring is gone.
  No deprecated alias kept for any of the above (clean break, pre-`0.1.0`,
  per established project policy). `schema_version` bumped `0.5.0` →
  `0.6.0`. Verified end-to-end against real ChEMBL/ORD/SynRXN corpora:
  `overall.difficulty` is now bit-for-bit identical across all three
  corpora (and the no-corpus default) for every molecule in round 19/20's
  15-molecule and 500-probe panels, while `fragment_precedent.signed_signal`
  still genuinely differs per corpus for 499/500 probes — the signal is
  untouched, only its influence on scoring is gone. Plain pyridine, which
  round 20 found swinging between `LikelyAccessible` and
  `HighlyChallenging` purely from corpus choice, now scores
  `LikelyAccessible` unconditionally. **Tradeoff, not a free win:** this
  reopens the `size_topology`/`functional_group_liability`
  over-penalization of common building blocks (e.g. dodecane, aspirin)
  that `fragment_precedent` used to correct through round 20 — round 21
  trades that milder, corpus-invariant problem for removing the riskier,
  corpus-dependent one. No formula, weight, or cap constant was changed.
  Full verification data in `tasks/upstream_and_corpus_research.md`
  Part 8 (gitignored); durable summary in `rules.rs`'s "Fragment
  precedent" section and `docs/architecture.md`'s roadmap (item 5).
- **`fragment_rarity` renamed to `fragment_precedent`** (round 18, API/
  schema semantic cleanup — no scoring formula change). The component
  argues difficulty both up (weakly precedented fragments) and down
  (strongly precedented ones), so "rarity detector" — a one-directional
  name — undersold what it measures.
  **Migration from `0.1.0-alpha.2`:**
  - `ComponentScores.fragment_rarity` → `ComponentScores.fragment_precedent`
  - `FindingCode::FragmentRarityHigh` → `FindingCode::FragmentPrecedentWeak`
    (`FindingCode::FragmentPrecedentStrong` is unchanged — the two are
    now a matched pair)
  - `components::fragment_rarity` (internal module) →
    `components::fragment_precedent`
  - `Provenance.model_version: Option<String>` →
    `Provenance.fragment_corpus: Option<FragmentCorpusProvenance>` (see
    below — a superset, not a same-shape rename)
  - `AnalysisConfig.fragment_model`/`FragmentModelConfig` are **unchanged**
    — audited and deliberately kept: that name was already generic ("config
    for whatever fragment-level model is configured"), not rarity-specific,
    so renaming it to something precedent-specific would have narrowed it
    for no benefit. See `config.rs`'s doc comment for the full reasoning.
  - `--fragment-corpus` (CLI flag) is **unchanged** — it names the corpus
    artifact itself, which was never called "rarity."
  No deprecated alias kept for any of the above (clean break, pre-`0.1.0`,
  per project policy). `schema_version` bumped `0.4.0` → `0.5.0`.
- **Corpus-domain provenance contract added** (round 18, AGENTS.md §5.4).
  `tools/build-fragment-corpus`'s `manifest.json` now carries a required
  `corpus_domain` block (`source_name`, `domain`, `synthesis_focused`,
  `description`), set via new required CLI flags
  (`--corpus-domain-name`/`--corpus-domain`/`--corpus-synthesis-focused`/
  `--corpus-domain-description` — required, not defaulted: guessing a
  domain would defeat the point). Also new: `fragment_definition_version`,
  `reference_distribution_version`, `mean_document_frequency`,
  `median_document_frequency`, `reference_distribution_definition`,
  `reference_distribution_quantiles` (named q01–q99 convenience subset of
  the full 1001-point grid). Every report produced against a configured
  corpus now carries this in the new `Provenance.fragment_corpus`
  (`FragmentCorpusProvenance`), so a report can be traced back to which
  corpus *and which chemical domain* produced its `fragment_precedent`
  signal — "rare in ChEMBL" and "hard to synthesize" are now traceably
  distinct claims, not implicitly conflated. Deliberately provenance-only
  this round: `synthesis_focused: false` does not lower a score, reduce
  confidence, or refuse the corpus — deciding whether/how scoring should
  react to it is future work, once a synthesis-focused corpus exists to
  compare against. `yomitoki::RULESET_VERSION` re-exported
  (`#[doc(hidden)]`) so the build tool can record which ruleset was current
  at build time. Corpora built before round 18 (missing `corpus_domain`)
  are rejected by `FragmentCorpus::load_dir` with no fallback, same
  treatment as round 17's `reference_distribution` requirement — rebuild
  with the current tool.

### Added

- `--exclude-smiles-file <path>` flag on `tools/build-fragment-corpus`
  (round 19): excludes canonical-SMILES matches from a corpus before
  dedup/`--limit`/frequency counting, for leave-one-out validation — a
  molecule under test must not be able to inflate its own precedent score
  by being present in the reference corpus it's scored against. Reported
  per-source in the manifest (`records_excluded_by_list`) and at the
  manifest level (`exclude_smiles_file`), so a corpus self-documents
  whether/how exclusion was applied.

### Validated

- **Cross-corpus validation of `fragment_precedent` (round 19) — result:
  CONDITIONAL GO, `GeneralOrganic` keeps the signal in `overall.difficulty`
  unchanged.** Built a second, matched-size (200,000-molecule) reference
  corpus from the Open Reaction Database (a genuinely synthesis-focused
  source, unlike ChEMBL) and re-ran the validation panel leakage
  -controlled against both. Confirmed: the common/simple panel never
  regresses in either corpus; ring/stereo structural burden is provably
  unaffected by which corpus is configured; no formula or weight was
  retuned in response to this round's data. Also confirmed a real but
  negligible-at-scale leakage issue in the existing ChEMBL corpus build
  (8 of 15 panel molecules were present in it; leakage-free re-measurement
  changed `overall.difficulty` by <0.0001 in every case — not a correction
  to any shipped number). The corpus-domain-bias caveat first reported in
  round 17 is now confirmed *heterogeneous* across structural motifs
  (some cases improve sharply under a synthesis-focused corpus, some don't
  move, one gets worse) rather than resolved by "use a better corpus" —
  see `rules.rs`'s "Fragment precedent" section and
  `tasks/upstream_and_corpus_research.md` Part 6 (gitignored) for the full
  data. The ORD-derived corpus itself is CC-BY-SA-4.0 (ShareAlike) and is
  not bundled with this crate — local validation artifact only this round.

## [0.1.0-alpha.2] - 2026-08-10

Second public preview. Publishes the round-17 `fragment_rarity` redesign
below — still a pre-release, not the completed v0.1 scope. `fragment_rarity`
is a working, corpus-validated correction mechanism now, but is likely to
be renamed (`fragment_precedent`) before a non-alpha `0.1.0`, since
"rarity detector" no longer describes what the component does (it argues
difficulty both up *and* down, not just up) — see `docs/architecture.md`
for the full reasoning. `0.1.0` is also waiting on a corpus-domain
provenance contract in the manifest and validation against at least one
synthesis-focused reference corpus (ChEMBL alone is bioactivity-biased,
not calibrated for synthetic precedent) before the `GeneralOrganic`
profile's contract is considered settled.

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
  minimum; see `rules.rs`'s "Fragment rarity" section). New
  `FindingCode::FragmentRarityHigh`/`FragmentPrecedentStrong`, new
  `SuggestionCode::IncreaseFragmentPrecedent` derivation, new
  `YomitokiError::ModelLoadError`, new `Provenance.model_version` field.
  New `chematic` `fp` feature dependency.
  No corpus-distribution decision has been made yet (the `yomitoki-core`/
  `yomitoki-models`/`yomitoki-data` split §5.4 sketches, vs. a
  feature-flagged external file) — build one locally with
  `tools/build-fragment-corpus` in the meantime.
  Round 16 found the initial formula (`WEIGHT * (1.0 -
  mean_document_frequency)`, added unconditionally to difficulty) confirmed
  broken by end-to-end testing against a real corpus, not merely untuned:
  it made the two documented false positives it exists to correct *worse*
  (aspirin's `overall.difficulty` `0.273` → `0.428`; dodecane's `0.068` →
  `0.227`). Round 17 redesigned it as a corpus-relative signed-precedent
  formula (percentile of the molecule's mean document frequency within the
  corpus's own distribution, `FragmentCorpus::percentile_rank` against a
  1001-point quantile grid computed during corpus build and stored as
  `manifest.json`'s `reference_distribution`), with precedent support
  capped in `analyze::analyze` at `size_topology`'s plus
  `functional_group_liability`'s combined contribution so strong fragment
  precedent can never erase `ring_topology`/`stereochemical_burden`
  burden. `ComponentScore.contribution` changed from `ProbabilityLikeScore`
  to a signed `f64` to represent this (a schema-breaking change, accepted
  pre-`0.1.0`). `SynthesizabilityReport.dominant_supports` is now
  genuinely populated (previously always empty), with a new
  `dominant_supports()` accessor. Confirmed end-to-end against the real
  200k-molecule corpus: aspirin `0.273` → `0.095`, paracetamol `0.243` →
  `0.095`, dodecane `0.068` → `0.000` — all three documented target cases
  now move in the intended direction. Known caveat, reported rather than
  hidden: some structurally-legitimate molecules (caffeine, bridged/spiro
  ring systems, stereocenter-dense cores) score *harder* against the
  ChEMBL corpus specifically, due to corpus-domain bias (ChEMBL is a
  bioactivity corpus, not a synthesis-focused one) — see `rules.rs`'s
  "Fragment rarity" section for the full before/after numbers.
  `ruleset_version` bumped to 0.8.0 then 0.9.0, `schema_version` to 0.4.0
  (the `ComponentScore.contribution` type change above is itself a schema
  change).
- `--fragment-corpus <dir>` CLI flag (`yomitoki analyze`), loads a
  `tools/build-fragment-corpus` output directory once before analyzing any
  molecule and enables `fragment_rarity` for the run. Omitted, behavior is
  unchanged from before this flag existed.

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
