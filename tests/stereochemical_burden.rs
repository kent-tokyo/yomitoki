//! Stereochemical burden component behavior. Fixtures were confirmed
//! against chematic's actual `stereo_completeness` output before being used
//! here (see `tasks/lessons.md` for the general practice).

use yomitoki::{AnalysisConfig, FindingCode, analyze_smiles};

fn stereo_difficulty(smiles: &str) -> f64 {
    let config = AnalysisConfig::default();
    let report = analyze_smiles(smiles, &config).expect("valid SMILES");
    report
        .components
        .stereochemical_burden
        .expect("stereochemical_burden always runs")
        .normalized
        .value()
}

#[test]
fn molecule_with_no_stereocenters_has_zero_burden() {
    assert_eq!(stereo_difficulty("CCO"), 0.0);
}

#[test]
fn specified_and_unspecified_stereocenters_burden_equally() {
    // A stereocenter needs the same synthetic control whether or not the
    // input SMILES wrote out its configuration — burden is about the
    // molecule, not the input string. Confidence (not burden) is where
    // "unspecified" matters; see components/applicability.rs.
    let unspecified = stereo_difficulty("CC(N)C(=O)O"); // alanine, no @ annotation
    let specified = stereo_difficulty("C[C@H](N)C(=O)O"); // alanine, specified
    assert_eq!(unspecified, specified);
    assert!(unspecified > 0.0);
}

#[test]
fn additional_stereocenters_do_not_reduce_burden() {
    // AGENTS.md §14.1: "additional stereocenters do not reduce stereo
    // burden" — direct metamorphic property from the spec's own list.
    let one_center = stereo_difficulty("CC(N)C(=O)O"); // 1 unspecified center
    let two_centers = stereo_difficulty("CC(N)C(O)C(Cl)C"); // 2 unspecified centers
    assert!(
        two_centers >= one_center,
        "one={one_center} two={two_centers}"
    );
}

#[test]
fn stereocenter_count_finding_present_when_centers_exist() {
    let config = AnalysisConfig::default();
    let report = analyze_smiles("CC(N)C(=O)O", &config).expect("valid SMILES");
    assert!(
        report
            .findings
            .iter()
            .any(|f| f.code == FindingCode::StereoCenterCount)
    );
}

#[test]
fn low_density_molecule_does_not_trigger_density_finding() {
    let config = AnalysisConfig::default();
    // 1 center / 6 atoms = 0.167, below the 0.25 density threshold.
    let report = analyze_smiles("CC(N)C(=O)O", &config).expect("valid SMILES");
    assert!(
        !report
            .findings
            .iter()
            .any(|f| f.code == FindingCode::StereoDensityHigh)
    );
}

#[test]
fn high_density_molecule_triggers_density_finding() {
    let config = AnalysisConfig::default();
    // 4 unspecified centers / 12 atoms = 0.333, above the 0.25 threshold.
    let report = analyze_smiles("CC(O)C(N)C(C)C(O)C(N)C", &config).expect("valid SMILES");
    assert!(
        report
            .findings
            .iter()
            .any(|f| f.code == FindingCode::StereoDensityHigh)
    );
}

#[test]
fn negatively_charged_atom_does_not_panic_and_reports_zero_with_a_finding() {
    // Regression test for a real bug: chematic's stereo_completeness
    // overflows (panics in debug builds) on any negatively charged atom
    // (chematic issue #267). This must not panic, must not silently claim
    // zero stereocenters, and must carry a finding saying why.
    let config = AnalysisConfig::default();
    let report = analyze_smiles("CC(=O)[O-]", &config).expect("acetate");
    assert_eq!(
        report
            .components
            .stereochemical_burden
            .expect("stereochemical_burden always runs")
            .normalized
            .value(),
        0.0
    );
    assert!(
        report
            .findings
            .iter()
            .any(|f| f.code == FindingCode::StereoAnalysisSkipped)
    );
}

#[test]
fn molecule_without_stereocenters_triggers_no_stereo_findings() {
    let config = AnalysisConfig::default();
    let report = analyze_smiles("C1CC2CCC1C2", &config).expect("norbornane"); // bridged, achiral
    assert!(
        !report
            .findings
            .iter()
            .any(|f| f.code == FindingCode::StereoCenterCount)
    );
    assert!(
        !report
            .findings
            .iter()
            .any(|f| f.code == FindingCode::StereoDensityHigh)
    );
}
