//! `analyze_batch` (AGENTS.md §18): input order is preserved, and batch
//! results must be identical to calling `analyze` on each molecule alone —
//! it's a convenience/parallelization-ready entry point, not a different
//! code path with its own semantics.

use yomitoki::{AnalysisConfig, analyze, analyze_batch};

fn mol(smiles: &str) -> chematic::core::Molecule {
    chematic::smiles::parse(smiles).unwrap_or_else(|e| panic!("parse '{smiles}': {e}"))
}

#[test]
fn empty_input_returns_empty_output() {
    let config = AnalysisConfig::default();
    let results = analyze_batch(&[], &config);
    assert!(results.is_empty());
}

#[test]
fn results_match_individual_analyze_calls_in_order() {
    let config = AnalysisConfig::default();
    let smiles = [
        "CCO",                    // ethanol
        "C1CC2CCC1C2",            // bridged (norbornane)
        "CC(O)C(N)C(C)C(O)C(N)C", // stereocenter-dense
        "C1CO1",                  // epoxide (Brenk alert)
    ];
    let molecules: Vec<_> = smiles.iter().map(|s| mol(s)).collect();

    let batch_results = analyze_batch(&molecules, &config);
    assert_eq!(batch_results.len(), molecules.len());

    for (i, molecule) in molecules.iter().enumerate() {
        let individual = analyze(molecule, &config).expect("analyze never fails on a parsed mol");
        let batched = batch_results[i]
            .as_ref()
            .expect("analyze never fails on a parsed mol");
        assert_eq!(
            batched, &individual,
            "batch result at index {i} ({}) diverged from an individual analyze() call",
            smiles[i]
        );
    }
}

#[test]
fn a_single_molecules_result_does_not_depend_on_its_neighbors() {
    // The same molecule, analyzed alone vs. alongside very different
    // molecules on either side, must score identically — batching must not
    // leak state between molecules.
    let config = AnalysisConfig::default();
    let target = mol("CC(=O)Oc1ccccc1C(=O)O"); // aspirin

    let alone = analyze_batch(std::slice::from_ref(&target), &config);
    let surrounded = analyze_batch(
        &[
            mol("C1CCCCCCCC1"), // 9-membered macrocycle
            target.clone(),
            mol("Cn1cnc2c1c(=O)n(c(=O)n2C)C"), // caffeine
        ],
        &config,
    );

    assert_eq!(alone[0].as_ref().unwrap(), surrounded[1].as_ref().unwrap());
}
