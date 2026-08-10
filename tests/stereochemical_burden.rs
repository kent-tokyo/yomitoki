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
fn negatively_charged_atom_does_not_panic() {
    // Regression test for a real, now-fixed upstream bug: chematic's
    // stereo_completeness used to overflow (panic in debug builds) on any
    // negatively charged atom (chematic issue #267), worked around here
    // until chematic 0.13.0 fixed it directly. Acetate has zero real
    // stereocenters, so zero burden here is a genuine computed result,
    // not the old hardcoded fallback -- StereoAnalysisSkipped no longer
    // fires at all (see the next test for a case that would have hidden
    // real stereocenters under the old fallback).
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
        !report
            .findings
            .iter()
            .any(|f| f.code == FindingCode::StereoAnalysisSkipped)
    );
}

#[test]
fn negatively_charged_atom_with_a_real_stereocenter_now_gets_real_burden() {
    // Alaninate: a specified stereocenter AND a negative charge. Before
    // chematic 0.13.0's fix, the old guard forced this to report zero
    // burden unconditionally -- a real molecule's real stereocenter was
    // silently invisible to overall.difficulty for every negatively
    // charged input. Now it must match its neutral-acid form exactly
    // (the charge is chemically irrelevant to the alpha-carbon's
    // configuration).
    let charged = stereo_difficulty("C[C@@H](N)C(=O)[O-]");
    let neutral = stereo_difficulty("C[C@@H](N)C(=O)O");
    assert_eq!(charged, neutral);
    assert!(charged > 0.0);
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
