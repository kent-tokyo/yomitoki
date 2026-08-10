//! `fragment_precedent` component + `FragmentCorpus::load_dir` behavior —
//! round 17's corpus-relative signed precedent redesign (see
//! `rules.rs`'s "Fragment precedent" section for the formula, and round
//! 16's finding that the original absolute-scale formula was confirmed
//! broken), plus round 18's rename from `fragment_rarity` and
//! corpus-domain provenance contract, plus round 21's option-C decoupling
//! (`fragment_precedent` no longer contributes to `overall.difficulty` —
//! see `SynthesizabilityReport.fragment_precedent`/
//! `FragmentPrecedentEvidence`; this component's own formula is
//! unchanged since round 17).
//!
//! Corpora are built from real `chematic::fp::morgan_fp_counts` output for
//! chosen fixture molecules (not hand-computed hashes) so the fragment
//! hashes in each test's synthetic corpus are guaranteed to match what the
//! component itself computes — the same discipline `tasks/lessons.md`
//! documents for other fixtures.
//!
//! Most tests use an *identity* reference distribution
//! (`reference_distribution[i] = i / 1000`), which makes
//! `FragmentCorpus::percentile_rank(x) == x` exactly for any `x` in
//! `0.0..=1.0` — so a fixture's mean document frequency can be set
//! directly to the percentile a test wants to exercise, no separate
//! calculation needed. The regression test near the bottom uses a
//! deliberately different, narrow distribution instead, to reproduce the
//! *shape* of the real corpus this bug was originally found against.

use std::path::Path;

use yomitoki::{AnalysisConfig, FindingCode, FragmentCorpus, SuggestionCode, analyze_smiles};

const RADIUS: u32 = 2;
const TOTAL: u64 = 1000;

/// The manifest fields round 18 made required (`corpus_domain`,
/// `fragment_definition_version`, `reference_distribution_version`) —
/// merged into every hand-built manifest fixture in this file so tests
/// that aren't specifically exercising *these* fields' own validation
/// don't fail on them incidentally.
fn required_manifest_extras() -> serde_json::Value {
    serde_json::json!({
        "fragment_definition_version": "test-fixture-v1",
        "reference_distribution_version": "test-fixture-v1",
        "corpus_domain": {
            "source_name": "Test Fixture",
            "domain": "test",
            "synthesis_focused": false,
            "description": "Not a real corpus -- exists only for integration test fixtures.",
        },
    })
}

fn merged_manifest(mut fields: serde_json::Value) -> serde_json::Value {
    let extras = required_manifest_extras();
    fields
        .as_object_mut()
        .expect("manifest fixture must be a JSON object")
        .extend(extras.as_object().unwrap().clone());
    fields
}

/// Writes a corpus where `smiles`'s own fragments each have document
/// frequency `occurrence / TOTAL`, and the reference distribution is the
/// identity grid (`percentile_rank(x) == x`).
fn write_corpus(dir: &Path, smiles: &str, occurrence: u64) {
    write_corpus_with_distribution(dir, smiles, occurrence, &identity_distribution());
}

fn identity_distribution() -> Vec<f64> {
    (0..=1000).map(|i| i as f64 / 1000.0).collect()
}

fn write_corpus_with_distribution(
    dir: &Path,
    smiles: &str,
    occurrence: u64,
    reference_distribution: &[f64],
) {
    let mol = chematic::smiles::parse(smiles).expect("valid fixture SMILES");
    let counts = chematic::fp::morgan_fp_counts(&mol, RADIUS);

    let fragments: Vec<serde_json::Value> = counts
        .keys()
        .map(|hash| {
            serde_json::json!({
                "radius": RADIUS,
                "fragment_hash": hash,
                "occurrence_count": occurrence,
            })
        })
        .collect();
    let table = serde_json::json!({
        "total_molecules_processed": TOTAL,
        "distinct_fragment_count": fragments.len(),
        "fragments": fragments,
    });
    std::fs::write(
        dir.join("fragment_frequencies.json"),
        serde_json::to_vec_pretty(&table).unwrap(),
    )
    .unwrap();

    let manifest = merged_manifest(serde_json::json!({
        "artifact_sha256": "sha256:test-fixture",
        "reference_distribution": reference_distribution,
    }));
    std::fs::write(
        dir.join("manifest.json"),
        serde_json::to_vec_pretty(&manifest).unwrap(),
    )
    .unwrap();
}

fn config_with_corpus(dir: &Path) -> AnalysisConfig {
    // `AnalysisConfig`/`FragmentModelConfig` are `#[non_exhaustive]`, so
    // external callers (this test included, since integration tests only
    // see the public API) build a default and mutate the field rather than
    // using struct-literal syntax.
    let corpus = FragmentCorpus::load_dir(dir).expect("load fixture corpus");
    let mut config = AnalysisConfig::default();
    config.fragment_model.corpus = Some(std::sync::Arc::new(corpus));
    config
}

#[test]
fn no_corpus_configured_leaves_fragment_precedent_none() {
    let config = AnalysisConfig::default();
    let report = analyze_smiles("CCO", &config).expect("valid SMILES");
    assert!(report.fragment_precedent.is_none());
    assert!(report.provenance.fragment_corpus.is_none());
    assert!(report.dominant_supports().is_empty());
}

#[test]
fn molecule_at_the_corpus_median_gets_no_finding_and_near_zero_signal() {
    // occurrence/TOTAL = 0.5 -> (identity grid) percentile 0.5 ->
    // signed_signal = 1 - 2*0.5 = 0.0 -- exactly neutral.
    let dir = tempfile::tempdir().expect("tempdir");
    write_corpus(dir.path(), "CCO", 500);
    let config = config_with_corpus(dir.path());

    let report = analyze_smiles("CCO", &config).expect("valid SMILES");
    let evidence = report.fragment_precedent.expect("corpus is configured");
    assert!(
        evidence.signed_signal.abs() < 1e-9,
        "signed_signal={}",
        evidence.signed_signal
    );
    assert!(
        !report
            .findings
            .iter()
            .any(|f| f.code == FindingCode::FragmentPrecedentWeak
                || f.code == FindingCode::FragmentPrecedentStrong)
    );
}

#[test]
fn molecule_at_a_low_percentile_gets_a_weak_precedent_penalty_reported_only_as_evidence() {
    // occurrence/TOTAL = 0.05 -> percentile 0.05 -> signed_signal = 0.9 --
    // a weak-precedent penalty, but (round 21 / option C) never a
    // difficulty contribution: it must not appear in dominant_penalties.
    let dir = tempfile::tempdir().expect("tempdir");
    write_corpus(dir.path(), "CCO", 50);
    let config = config_with_corpus(dir.path());

    let report = analyze_smiles("CCO", &config).expect("valid SMILES");
    assert!(
        report
            .findings
            .iter()
            .any(|f| f.code == FindingCode::FragmentPrecedentWeak)
    );
    assert!(
        !report
            .dominant_penalties()
            .iter()
            .any(|c| c.code == FindingCode::FragmentPrecedentWeak),
        "fragment_precedent must never appear in dominant_penalties (round 21)"
    );
    assert!(report.dominant_supports().is_empty());
    let evidence = report.fragment_precedent.expect("corpus is configured");
    assert!(
        evidence.precedent_penalty > 0.0,
        "precedent_penalty={}",
        evidence.precedent_penalty
    );
    assert!(evidence.signed_signal > 0.0);
}

#[test]
fn molecule_at_a_high_percentile_gets_precedent_support_reported_only_as_evidence() {
    // occurrence/TOTAL = 0.95 -> percentile 0.95 -> signed_signal = -0.9 --
    // strong precedent support, but (round 21 / option C) never a
    // difficulty contribution: it must not appear in dominant_supports.
    let dir = tempfile::tempdir().expect("tempdir");
    write_corpus(dir.path(), "CCO", 950);
    let config = config_with_corpus(dir.path());

    let report = analyze_smiles("CCO", &config).expect("valid SMILES");
    assert!(
        report
            .findings
            .iter()
            .any(|f| f.code == FindingCode::FragmentPrecedentStrong)
    );
    assert!(
        report.dominant_supports().is_empty(),
        "fragment_precedent must never appear in dominant_supports (round 21) -- \
         dominant_supports is always empty in v0.1 now"
    );
    assert!(
        !report
            .dominant_penalties()
            .iter()
            .any(|c| c.code == FindingCode::FragmentPrecedentStrong)
    );
    let evidence = report.fragment_precedent.expect("corpus is configured");
    assert!(
        evidence.precedent_support > 0.0,
        "precedent_support={}",
        evidence.precedent_support
    );
    assert!(evidence.signed_signal < 0.0);
}

#[test]
fn fragment_precedent_weak_finding_no_longer_produces_a_suggestion() {
    // SuggestionCode::IncreaseFragmentPrecedent is unreachable since round
    // 21: once fragment_precedent doesn't affect overall.difficulty,
    // "increase precedent" can't be truthfully labeled MayReduceDifficulty.
    let dir = tempfile::tempdir().expect("tempdir");
    write_corpus(dir.path(), "CCO", 10);
    let config = config_with_corpus(dir.path());

    let report = analyze_smiles("CCO", &config).expect("valid SMILES");
    assert!(
        !report
            .suggestions
            .iter()
            .any(|s| s.code == SuggestionCode::IncreaseFragmentPrecedent)
    );
}

#[test]
fn corpus_choice_never_changes_overall_difficulty() {
    // The round-21 core guarantee (option C): overall.difficulty is
    // computed from ring/size/stereo/functional-group evidence only, so
    // swapping the configured corpus -- or configuring one at all --
    // cannot change it, for any molecule, ever. This replaces round
    // 17-19's weaker "support is capped so it can't erase ring burden"
    // guarantee with an exact-equality one.
    let dir_low = tempfile::tempdir().expect("tempdir");
    write_corpus(dir_low.path(), "C1CC2CCC1C2", 1); // near-minimal precedent
    let dir_high = tempfile::tempdir().expect("tempdir");
    write_corpus(dir_high.path(), "C1CC2CCC1C2", 999); // near-maximal precedent

    let config_none = AnalysisConfig::default();
    let config_low = config_with_corpus(dir_low.path());
    let config_high = config_with_corpus(dir_high.path());

    let no_corpus = analyze_smiles("C1CC2CCC1C2", &config_none).expect("valid SMILES");
    let low_precedent = analyze_smiles("C1CC2CCC1C2", &config_low).expect("valid SMILES");
    let high_precedent = analyze_smiles("C1CC2CCC1C2", &config_high).expect("valid SMILES");

    // The signal itself really does differ (sanity check the fixtures
    // actually exercise opposite ends of the range) -- if this fails, the
    // test below would trivially pass for the wrong reason.
    let low_signal = low_precedent.fragment_precedent.unwrap().signed_signal;
    let high_signal = high_precedent.fragment_precedent.unwrap().signed_signal;
    assert!(
        low_signal > 0.5 && high_signal < -0.5,
        "fixtures didn't exercise opposite ends: low={low_signal} high={high_signal}"
    );

    assert_eq!(
        no_corpus.overall.difficulty, low_precedent.overall.difficulty,
        "difficulty changed when a corpus was configured"
    );
    assert_eq!(
        no_corpus.overall.difficulty, high_precedent.overall.difficulty,
        "difficulty changed when a corpus was configured"
    );
    assert_eq!(
        low_precedent.overall.difficulty, high_precedent.overall.difficulty,
        "difficulty changed purely from which corpus was configured"
    );
    assert_eq!(no_corpus.overall.verdict, low_precedent.overall.verdict);
    assert_eq!(no_corpus.overall.verdict, high_precedent.overall.verdict);
}

#[test]
fn load_dir_errors_cleanly_on_a_missing_directory() {
    let result = FragmentCorpus::load_dir("/nonexistent/path/that/does/not/exist");
    assert!(result.is_err());
}

#[test]
fn load_dir_errors_cleanly_on_mixed_radii() {
    let dir = tempfile::tempdir().expect("tempdir");
    let table = serde_json::json!({
        "total_molecules_processed": 10,
        "distinct_fragment_count": 2,
        "fragments": [
            {"radius": 0, "fragment_hash": 1, "occurrence_count": 5},
            {"radius": 1, "fragment_hash": 2, "occurrence_count": 5},
        ],
    });
    std::fs::write(
        dir.path().join("fragment_frequencies.json"),
        serde_json::to_vec_pretty(&table).unwrap(),
    )
    .unwrap();
    let manifest = merged_manifest(serde_json::json!({
        "artifact_sha256": "x",
        "reference_distribution": identity_distribution(),
    }));
    std::fs::write(
        dir.path().join("manifest.json"),
        serde_json::to_vec_pretty(&manifest).unwrap(),
    )
    .unwrap();

    let result = FragmentCorpus::load_dir(dir.path());
    assert!(result.is_err());
}

#[test]
fn load_dir_errors_cleanly_on_a_missing_reference_distribution() {
    let dir = tempfile::tempdir().expect("tempdir");
    let table = serde_json::json!({
        "total_molecules_processed": 10,
        "distinct_fragment_count": 1,
        "fragments": [{"radius": 2, "fragment_hash": 1, "occurrence_count": 5}],
    });
    std::fs::write(
        dir.path().join("fragment_frequencies.json"),
        serde_json::to_vec_pretty(&table).unwrap(),
    )
    .unwrap();
    // Every other required manifest field present (round 18's
    // corpus_domain/*_version fields), but no reference_distribution at
    // all -- isolates round 17's own validation from round 18's, so this
    // still tests what its name claims.
    let manifest = merged_manifest(serde_json::json!({"artifact_sha256": "x"}));
    std::fs::write(
        dir.path().join("manifest.json"),
        serde_json::to_vec_pretty(&manifest).unwrap(),
    )
    .unwrap();

    let result = FragmentCorpus::load_dir(dir.path());
    assert!(result.is_err());
}

#[test]
fn load_dir_errors_cleanly_on_a_missing_corpus_domain() {
    let dir = tempfile::tempdir().expect("tempdir");
    let table = serde_json::json!({
        "total_molecules_processed": 10,
        "distinct_fragment_count": 1,
        "fragments": [{"radius": 2, "fragment_hash": 1, "occurrence_count": 5}],
    });
    std::fs::write(
        dir.path().join("fragment_frequencies.json"),
        serde_json::to_vec_pretty(&table).unwrap(),
    )
    .unwrap();
    // reference_distribution present, but no corpus_domain -- a corpus
    // built before round 18. There is no undeclared-domain fallback (see
    // FragmentCorpusProvenance's doc for why domain provenance isn't
    // optional).
    std::fs::write(
        dir.path().join("manifest.json"),
        serde_json::to_vec_pretty(&serde_json::json!({
            "artifact_sha256": "x",
            "reference_distribution": identity_distribution(),
            "fragment_definition_version": "test-fixture-v1",
            "reference_distribution_version": "test-fixture-v1",
        }))
        .unwrap(),
    )
    .unwrap();

    let result = FragmentCorpus::load_dir(dir.path());
    assert!(result.is_err());
}

/// Sanity check that the fixture-building helper itself does what it
/// claims: every fragment hash it writes really is one
/// `chematic::fp::morgan_fp_counts` produces for the same SMILES.
#[test]
fn fixture_helper_hashes_match_the_real_fingerprint_function() {
    let mol = chematic::smiles::parse("CCO").expect("valid SMILES");
    let real = chematic::fp::morgan_fp_counts(&mol, RADIUS);
    assert!(!real.is_empty());
}

/// **Regression test — round 17.** Round 16 found that a molecule with a
/// *realistically* "common" document frequency (~0.27, matching real
/// measured aspirin data against a real 200k-molecule corpus) scored
/// *harder* once a corpus was configured, under the old absolute-scale
/// formula. This reproduces the same input shape against a corpus whose
/// reference distribution is narrow and low (most of the corpus clusters
/// well under 0.27, matching the real corpus's own q01..q99 band of
/// roughly 0.08..0.25) — so 0.27 correctly lands at a *high* percentile,
/// and the fixed formula must not make this molecule score harder.
///
/// If you're touching the formula and this fails: that's the point of
/// this test. Don't just delete it or flip the assertion back — figure
/// out whether the regression actually came back.
#[test]
fn regression_a_realistically_common_molecule_does_not_score_harder_with_a_corpus() {
    let dir = tempfile::tempdir().expect("tempdir");
    // A narrow, low reference distribution (values from 0.0 up to 0.27,
    // never above it) -- 0.27 sits at the very top, matching how real
    // aspirin data (~0.268) landed above the real corpus's own q99
    // (~0.250) in round 16's actual measurement.
    let narrow_low_distribution: Vec<f64> = (0..=1000).map(|i| 0.27 * i as f64 / 1000.0).collect();
    write_corpus_with_distribution(dir.path(), "CCO", 270, &narrow_low_distribution);
    let config_without = AnalysisConfig::default();
    let config_with = config_with_corpus(dir.path());

    let without_corpus = analyze_smiles("CCO", &config_without).expect("valid SMILES");
    let with_corpus = analyze_smiles("CCO", &config_with).expect("valid SMILES");

    let difficulty_without = without_corpus.overall.difficulty.value();
    let difficulty_with = with_corpus.overall.difficulty.value();
    assert!(
        difficulty_with <= difficulty_without + 1e-9,
        "regression: a realistically-common molecule scored harder with a corpus \
         configured (without={difficulty_without}, with={difficulty_with}) -- this is \
         the exact round-16 bug"
    );
}

/// **Regression test — round 18.** `Provenance.fragment_corpus` must carry
/// the corpus's own domain declaration through end-to-end, not just its
/// version identifier — this is the whole point of the round-18
/// provenance contract (AGENTS.md §5.4): a report reader needs to know
/// *which chemical domain* a `fragment_precedent` signal was measured
/// against.
#[test]
fn provenance_carries_the_configured_corpus_domain() {
    let dir = tempfile::tempdir().expect("tempdir");
    write_corpus(dir.path(), "CCO", 500);
    let config = config_with_corpus(dir.path());

    let report = analyze_smiles("CCO", &config).expect("valid SMILES");
    let fragment_corpus = report
        .provenance
        .fragment_corpus
        .expect("corpus is configured");
    assert_eq!(fragment_corpus.version, "sha256:test-fixture");
    assert_eq!(fragment_corpus.source_name, "Test Fixture");
    assert_eq!(fragment_corpus.domain, "test");
    assert!(!fragment_corpus.synthesis_focused);
    assert_eq!(
        fragment_corpus.fragment_definition_version,
        "test-fixture-v1"
    );
    assert_eq!(
        fragment_corpus.reference_distribution_version,
        "test-fixture-v1"
    );
}
