//! End-to-end coverage of simplification suggestions (AGENTS.md §9), through
//! `analyze_smiles` rather than `suggestions::derive` directly (that's
//! covered by inline unit tests in `src/suggestions.rs`) — this file checks
//! the whole pipeline actually wires findings into suggestions.

use yomitoki::{AnalysisConfig, ExpectedEffect, SuggestionCode};

#[test]
fn bridged_ring_molecule_gets_a_replace_bridged_ring_suggestion() {
    let config = AnalysisConfig::default();
    let report = yomitoki::analyze_smiles("C1CC2CCC1C2", &config).expect("norbornane");
    assert!(
        report
            .suggestions
            .iter()
            .any(|s| s.code == SuggestionCode::ReplaceBridgedRingWithMonocyclicAnalog)
    );
}

#[test]
fn macrocyclic_molecule_gets_a_simplify_macrocycle_suggestion() {
    let config = AnalysisConfig::default();
    let report = yomitoki::analyze_smiles("C1CCCCCCCC1", &config).expect("9-membered ring");
    assert!(
        report
            .suggestions
            .iter()
            .any(|s| s.code == SuggestionCode::SimplifyMacrocyclicClosure)
    );
}

#[test]
fn stereo_dense_molecule_gets_a_reduce_density_suggestion() {
    let config = AnalysisConfig::default();
    let report =
        yomitoki::analyze_smiles("CC(O)C(N)C(C)C(O)C(N)C", &config).expect("stereo-dense fragment");
    assert!(
        report
            .suggestions
            .iter()
            .any(|s| s.code == SuggestionCode::ReduceStereocenterDensity)
    );
}

#[test]
fn plain_molecule_with_no_matching_findings_gets_no_suggestions() {
    let config = AnalysisConfig::default();
    let report = yomitoki::analyze_smiles("CCO", &config).expect("ethanol");
    assert!(report.suggestions.is_empty());
}

#[test]
fn every_suggestion_is_heuristic_never_a_guarantee() {
    // AGENTS.md §9: never assert a suggestion is certain to help.
    let config = AnalysisConfig::default();
    for smiles in ["C1CC2CCC1C2", "C1CCCCCCCC1", "CC(O)C(N)C(C)C(O)C(N)C"] {
        let report = yomitoki::analyze_smiles(smiles, &config).expect("valid SMILES");
        for suggestion in &report.suggestions {
            assert_eq!(
                suggestion.expected_effect,
                ExpectedEffect::MayReduceDifficulty
            );
            assert!(suggestion.confidence.value() < 1.0);
        }
    }
}
