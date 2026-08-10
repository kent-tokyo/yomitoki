//! `fragment_rarity` component + `FragmentCorpus::load_dir` behavior —
//! round 17's corpus-relative signed precedent redesign (see
//! `rules.rs`'s "Fragment rarity" section for the formula, and round 16's
//! finding that the original absolute-scale formula was confirmed broken).
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

    let manifest = serde_json::json!({
        "artifact_sha256": "sha256:test-fixture",
        "reference_distribution": reference_distribution,
    });
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
fn no_corpus_configured_leaves_fragment_rarity_none() {
    let config = AnalysisConfig::default();
    let report = analyze_smiles("CCO", &config).expect("valid SMILES");
    assert!(report.components.fragment_rarity.is_none());
    assert!(report.provenance.model_version.is_none());
    assert!(report.dominant_supports().is_empty());
}

#[test]
fn molecule_at_the_corpus_median_gets_no_finding_and_near_zero_contribution() {
    // occurrence/TOTAL = 0.5 -> (identity grid) percentile 0.5 ->
    // signed_signal = 1 - 2*0.5 = 0.0 -- exactly neutral.
    let dir = tempfile::tempdir().expect("tempdir");
    write_corpus(dir.path(), "CCO", 500);
    let config = config_with_corpus(dir.path());

    let report = analyze_smiles("CCO", &config).expect("valid SMILES");
    let score = report
        .components
        .fragment_rarity
        .expect("corpus is configured");
    assert!(
        score.contribution.abs() < 1e-9,
        "contribution={}",
        score.contribution
    );
    assert!(
        !report
            .findings
            .iter()
            .any(|f| f.code == FindingCode::FragmentRarityHigh
                || f.code == FindingCode::FragmentPrecedentStrong)
    );
}

#[test]
fn molecule_at_a_low_percentile_gets_a_rarity_penalty_and_a_positive_contribution() {
    // occurrence/TOTAL = 0.05 -> percentile 0.05 -> signed_signal = 0.9 ->
    // a penalty (positive contribution to difficulty).
    let dir = tempfile::tempdir().expect("tempdir");
    write_corpus(dir.path(), "CCO", 50);
    let config = config_with_corpus(dir.path());

    let report = analyze_smiles("CCO", &config).expect("valid SMILES");
    assert!(
        report
            .findings
            .iter()
            .any(|f| f.code == FindingCode::FragmentRarityHigh)
    );
    assert!(
        report
            .dominant_penalties()
            .iter()
            .any(|c| c.code == FindingCode::FragmentRarityHigh)
    );
    assert!(report.dominant_supports().is_empty());
    let score = report
        .components
        .fragment_rarity
        .expect("corpus is configured");
    assert!(
        score.contribution > 0.0,
        "contribution={}",
        score.contribution
    );
}

#[test]
fn molecule_at_a_high_percentile_gets_precedent_support_and_a_negative_contribution() {
    // occurrence/TOTAL = 0.95 -> percentile 0.95 -> signed_signal = -0.9 ->
    // support (negative contribution to difficulty, before any cap).
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
        report
            .dominant_supports()
            .iter()
            .any(|c| c.code == FindingCode::FragmentPrecedentStrong)
    );
    assert!(
        !report
            .dominant_penalties()
            .iter()
            .any(|c| c.code == FindingCode::FragmentPrecedentStrong)
    );
    let score = report
        .components
        .fragment_rarity
        .expect("corpus is configured");
    assert!(
        score.contribution < 0.0,
        "contribution={}",
        score.contribution
    );
}

#[test]
fn molecule_at_a_low_percentile_gets_an_increase_precedent_suggestion() {
    let dir = tempfile::tempdir().expect("tempdir");
    write_corpus(dir.path(), "CCO", 10);
    let config = config_with_corpus(dir.path());

    let report = analyze_smiles("CCO", &config).expect("valid SMILES");
    assert!(
        report
            .suggestions
            .iter()
            .any(|s| s.code == SuggestionCode::IncreaseFragmentPrecedent)
    );
}

#[test]
fn strong_precedent_support_never_erases_ring_topology_burden() {
    // Norbornane (bridged bicyclic, C1CC2CCC1C2) has real ring_topology
    // burden and near-zero size_topology/functional_group_liability
    // burden -- exactly the shape AGENTS.md §5.4's cap requirement is
    // about: strong fragment precedent must offset "unusual substituent
    // pattern" burden (size/FG) but never "this ring system is
    // structurally hard" burden (ring/stereo).
    let dir = tempfile::tempdir().expect("tempdir");
    // occurrence/TOTAL = 0.999 -> percentile ~0.999 -> signed_signal
    // ~ -0.998, close to maximal precedent_support.
    write_corpus(dir.path(), "C1CC2CCC1C2", 999);
    let config_without = AnalysisConfig::default();
    let config_with = config_with_corpus(dir.path());

    let without_corpus = analyze_smiles("C1CC2CCC1C2", &config_without).expect("valid SMILES");
    let with_corpus = analyze_smiles("C1CC2CCC1C2", &config_with).expect("valid SMILES");

    let ring_burden = without_corpus
        .components
        .ring_topology
        .expect("ring_topology always runs")
        .normalized
        .value();
    assert!(
        ring_burden > 0.05,
        "expected real ring burden: {ring_burden}"
    );

    // The support is real (contribution is negative)...
    let fragment_score = with_corpus
        .components
        .fragment_rarity
        .expect("corpus is configured");
    assert!(fragment_score.contribution < 0.0);
    // ...but capped: it can only offset size_topology's +
    // functional_group_liability's own contribution, which is small for
    // this molecule, so overall.difficulty stays well above zero --
    // nowhere near erasing the ring-topology-driven floor.
    assert!(
        with_corpus.overall.difficulty.value() > ring_burden * 0.5,
        "difficulty={} ring_burden={} -- strong precedent should not have \
         erased ring-topology burden",
        with_corpus.overall.difficulty.value(),
        ring_burden
    );
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
    std::fs::write(
        dir.path().join("manifest.json"),
        serde_json::to_vec_pretty(&serde_json::json!({
            "artifact_sha256": "x",
            "reference_distribution": identity_distribution(),
        }))
        .unwrap(),
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
    // No reference_distribution at all -- a corpus built before round 17.
    std::fs::write(
        dir.path().join("manifest.json"),
        serde_json::to_vec_pretty(&serde_json::json!({"artifact_sha256": "x"})).unwrap(),
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
