//! Functional-group liability component behavior. Fixtures were confirmed
//! against chematic's actual `brenk_matches_detailed` output before being
//! used here (see `tasks/lessons.md` for the general practice).

use yomitoki::{AnalysisConfig, FindingCode, analyze_smiles};

fn fg_difficulty(smiles: &str) -> f64 {
    let config = AnalysisConfig::default();
    let report = analyze_smiles(smiles, &config).expect("valid SMILES");
    report
        .components
        .functional_group_liability
        .expect("functional_group_liability always runs")
        .normalized
        .value()
}

#[test]
fn molecule_with_no_brenk_alerts_has_zero_burden() {
    assert_eq!(fg_difficulty("CCO"), 0.0); // ethanol
    assert_eq!(fg_difficulty("c1ccccc1"), 0.0); // benzene
}

#[test]
fn epoxide_triggers_a_reactive_group_finding() {
    let config = AnalysisConfig::default();
    let report = analyze_smiles("C1CO1", &config).expect("valid SMILES");
    let finding = report
        .findings
        .iter()
        .find(|f| f.code == FindingCode::FunctionalGroupReactive)
        .expect("epoxide is a Brenk alert");
    assert!(finding.explanation.contains("epoxide"));
    assert!(!finding.atoms.is_empty());
    assert!(fg_difficulty("C1CO1") > 0.0);
}

#[test]
fn strained_three_membered_ring_triggers_a_finding() {
    // AGENTS.md §5.5's "strained motifs" example, covered by Brenk's own
    // strained_ring_three alert rather than a hand-rolled ring-size check.
    let config = AnalysisConfig::default();
    let report = analyze_smiles("C1CC1", &config).expect("cyclopropane");
    assert!(
        report
            .findings
            .iter()
            .any(|f| f.code == FindingCode::FunctionalGroupReactive)
    );
}

#[test]
fn molecule_with_multiple_alerts_has_more_findings_than_one_alert() {
    // CC(=O)Cl (acetyl chloride) triggers both acyl_halide and alkyl_halide
    // (the C-Cl bond matches both patterns) — more alerts than a
    // single-alert fixture like the epoxide above.
    let config = AnalysisConfig::default();
    let report = analyze_smiles("CC(=O)Cl", &config).expect("valid SMILES");
    let count = report
        .findings
        .iter()
        .filter(|f| f.code == FindingCode::FunctionalGroupReactive)
        .count();
    assert!(count >= 2, "expected >=2 alerts, got {count}");
}

#[test]
fn additional_alerts_do_not_reduce_burden() {
    let one_alert = fg_difficulty("C1CO1"); // epoxide only
    let more_alerts = fg_difficulty("CC(=O)Cl"); // acyl_halide + alkyl_halide
    assert!(
        more_alerts >= one_alert,
        "one={one_alert} more={more_alerts}"
    );
}

#[test]
fn norbornane_triggers_no_functional_group_alerts() {
    // Confirms this component doesn't regress the existing ring_topology
    // fixture used elsewhere (tests/ring_topology.rs, examples/basic.rs) —
    // norbornane is saturated and has no Brenk-flagged functional groups.
    assert_eq!(fg_difficulty("C1CC2CCC1C2"), 0.0);
}
