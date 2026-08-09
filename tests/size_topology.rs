//! Size/topology component behavior. Fixtures were confirmed against
//! chematic's actual descriptor output before being used here (see
//! `tasks/lessons.md` for the general practice).

use yomitoki::{AnalysisConfig, FindingCode, analyze_smiles};

fn size_difficulty(smiles: &str) -> f64 {
    let config = AnalysisConfig::default();
    let report = analyze_smiles(smiles, &config).expect("valid SMILES");
    report
        .components
        .size_topology
        .expect("size_topology always runs")
        .normalized
        .value()
}

#[test]
fn small_molecule_has_low_size_burden() {
    assert!(size_difficulty("CCO") < 0.05, "{}", size_difficulty("CCO"));
}

#[test]
fn extending_a_chain_never_decreases_size_burden() {
    // AGENTS.md §14.3 metamorphic property applied to size: a simple alkyl
    // extension must never make the molecule look easier.
    let short = size_difficulty("CCCC");
    let long = size_difficulty("CCCCCCCCCCCC");
    assert!(long >= short, "short={short} long={long}");
}

#[test]
fn large_molecular_weight_is_flagged() {
    let config = AnalysisConfig::default();
    // 39-membered ring: MW 547 (> the 500 Da threshold), 0 rotatable bonds
    // (ring bonds aren't rotatable) — isolates the MW finding from the
    // rotatable-bond finding.
    let report =
        analyze_smiles("C1CCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCC1", &config).expect("valid SMILES");
    assert!(
        report
            .findings
            .iter()
            .any(|f| f.code == FindingCode::SizeLargeMolecularWeight)
    );
    assert!(
        !report
            .findings
            .iter()
            .any(|f| f.code == FindingCode::SizeHighRotatableBondCount)
    );
}

#[test]
fn high_rotatable_bond_count_is_flagged() {
    let config = AnalysisConfig::default();
    // 14-carbon chain: 11 rotatable bonds (> the 10-bond threshold), MW
    // 198 (well under the 500 Da threshold) — isolates the rotatable-bond
    // finding from the MW finding.
    let report = analyze_smiles("CCCCCCCCCCCCCC", &config).expect("valid SMILES");
    assert!(
        report
            .findings
            .iter()
            .any(|f| f.code == FindingCode::SizeHighRotatableBondCount)
    );
    assert!(
        !report
            .findings
            .iter()
            .any(|f| f.code == FindingCode::SizeLargeMolecularWeight)
    );
}

#[test]
fn small_molecule_triggers_neither_size_finding() {
    let config = AnalysisConfig::default();
    let report = analyze_smiles("CCO", &config).expect("valid SMILES");
    assert!(
        !report
            .findings
            .iter()
            .any(|f| f.code == FindingCode::SizeLargeMolecularWeight)
    );
    assert!(
        !report
            .findings
            .iter()
            .any(|f| f.code == FindingCode::SizeHighRotatableBondCount)
    );
}
