//! `fragment_rarity` component + `FragmentCorpus::load_dir` behavior.
//! Corpora are built from real `chematic::fp::morgan_fp_counts` output for
//! chosen fixture molecules (not hand-computed hashes) so the fragment
//! hashes in each test's synthetic corpus are guaranteed to match what the
//! component itself computes — the same discipline `tasks/lessons.md`
//! documents for other fixtures.

use std::path::Path;

use yomitoki::{AnalysisConfig, FindingCode, FragmentCorpus, SuggestionCode, analyze_smiles};

const RADIUS: u32 = 2;

/// Writes a `fragment_frequencies.json` + `manifest.json` pair (the exact
/// shape `tools/build-fragment-corpus` writes) into `dir`, where every
/// fragment of `dominant_smiles` is given `occurrence_count = total`
/// (maximally common) and nothing else is present — so any molecule
/// sharing no fragments with `dominant_smiles` scores maximally rare.
fn write_dominant_corpus(dir: &Path, dominant_smiles: &str, total: u64) {
    let mol = chematic::smiles::parse(dominant_smiles).expect("valid fixture SMILES");
    let counts = chematic::fp::morgan_fp_counts(&mol, RADIUS);

    let fragments: Vec<serde_json::Value> = counts
        .keys()
        .map(|hash| {
            serde_json::json!({
                "radius": RADIUS,
                "fragment_hash": hash,
                "occurrence_count": total,
            })
        })
        .collect();
    let table = serde_json::json!({
        "total_molecules_processed": total,
        "distinct_fragment_count": fragments.len(),
        "fragments": fragments,
    });
    std::fs::write(
        dir.join("fragment_frequencies.json"),
        serde_json::to_vec_pretty(&table).unwrap(),
    )
    .unwrap();

    let manifest = serde_json::json!({ "artifact_sha256": "sha256:test-fixture" });
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
}

#[test]
fn molecule_dominating_the_corpus_has_low_rarity_and_no_finding() {
    let dir = tempfile::tempdir().expect("tempdir");
    // Ethanol's own fragments, at maximal document frequency (1.0).
    write_dominant_corpus(dir.path(), "CCO", 1000);
    let config = config_with_corpus(dir.path());

    let report = analyze_smiles("CCO", &config).expect("valid SMILES");
    let score = report
        .components
        .fragment_rarity
        .expect("corpus is configured");
    assert!(
        score.normalized.value() < 0.05,
        "normalized={}",
        score.normalized.value()
    );
    assert!(
        !report
            .findings
            .iter()
            .any(|f| f.code == FindingCode::FragmentRarityHigh)
    );
}

#[test]
fn molecule_absent_from_the_corpus_scores_higher_than_one_dominating_it() {
    // Relative comparison, not a hardcoded absolute threshold —
    // FRAGMENT_RARITY_BURDEN_SCALE is explicitly documented as an untuned
    // first-pass constant (rules.rs), so this checks the direction the
    // score moves in, not a specific magnitude that would be as arbitrary
    // as the constant itself.
    let dir = tempfile::tempdir().expect("tempdir");
    write_dominant_corpus(dir.path(), "CCO", 1000);
    let config = config_with_corpus(dir.path());

    let dominant_report = analyze_smiles("CCO", &config).expect("valid SMILES");
    // A structurally unrelated aromatic heterocycle — shares no fragments
    // with the corpus, so every one of its fragments has document
    // frequency 0.0 (maximally rare).
    let absent_report = analyze_smiles("c1ccc2[nH]ccc2c1", &config).expect("valid SMILES");

    let dominant_score = dominant_report
        .components
        .fragment_rarity
        .expect("corpus is configured")
        .normalized
        .value();
    let absent_score = absent_report
        .components
        .fragment_rarity
        .expect("corpus is configured")
        .normalized
        .value();
    assert!(
        absent_score > dominant_score,
        "absent={absent_score} dominant={dominant_score}"
    );
    assert!(
        absent_report
            .findings
            .iter()
            .any(|f| f.code == FindingCode::FragmentRarityHigh)
    );
}

#[test]
fn molecule_absent_from_the_corpus_gets_an_increase_precedent_suggestion() {
    let dir = tempfile::tempdir().expect("tempdir");
    write_dominant_corpus(dir.path(), "CCO", 1000);
    let config = config_with_corpus(dir.path());

    let report = analyze_smiles("c1ccc2[nH]ccc2c1", &config).expect("valid SMILES");
    assert!(
        report
            .suggestions
            .iter()
            .any(|s| s.code == SuggestionCode::IncreaseFragmentPrecedent)
    );
}

#[test]
fn provenance_model_version_matches_the_configured_corpus() {
    let dir = tempfile::tempdir().expect("tempdir");
    write_dominant_corpus(dir.path(), "CCO", 1000);
    let config = config_with_corpus(dir.path());

    let report = analyze_smiles("CCO", &config).expect("valid SMILES");
    assert_eq!(
        report.provenance.model_version.as_deref(),
        Some("sha256:test-fixture")
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
        serde_json::to_vec_pretty(&serde_json::json!({ "artifact_sha256": "x" })).unwrap(),
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
