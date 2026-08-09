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

#[test]
fn a_single_functional_group_cluster_is_not_dense() {
    // Confirmed via chematic::chem::identify_functional_groups directly:
    // ethanol is one connected C-O environment (count == 1) — the first
    // cluster is free (rules::FG_WEIGHT_PER_DISTINCT_GROUP), so this must
    // stay exactly the pre-existing zero-burden baseline, not just "low".
    assert_eq!(fg_difficulty("CCO"), 0.0);
}

#[test]
fn many_scattered_functional_groups_trigger_a_dense_finding() {
    // Pentaerythritol tetraacetate: 4 esters, each separated from the
    // others by the branched core's CH2 spacers — confirmed via
    // chematic::chem::identify_functional_groups directly to be 4 disjoint
    // clusters (above FG_DENSE_GROUP_COUNT_THRESHOLD = 3), unlike a single
    // fused/interconnected polyfunctional system (documented weak spot).
    let config = AnalysisConfig::default();
    let report = analyze_smiles("CC(=O)OCC(COC(C)=O)(COC(C)=O)COC(C)=O", &config)
        .expect("pentaerythritol tetraacetate");
    let finding = report
        .findings
        .iter()
        .find(|f| f.code == FindingCode::FunctionalGroupDense)
        .expect("4 disjoint ester clusters should trigger FunctionalGroupDense");
    assert_eq!(finding.evidence.value, Some(4.0));
    assert!(
        finding.atoms.is_empty(),
        "molecule-level finding, not tied to one atom region"
    );
}

#[test]
fn two_ordinary_functional_groups_do_not_trigger_a_dense_finding() {
    // Aspirin: phenol/ester + carboxylic acid, count == 2 — well under the
    // dense threshold, matching how ordinary drug-like molecules shouldn't
    // be flagged for "dense functionalization".
    let config = AnalysisConfig::default();
    let report = analyze_smiles("CC(=O)Oc1ccccc1C(=O)O", &config).expect("aspirin");
    assert!(
        !report
            .findings
            .iter()
            .any(|f| f.code == FindingCode::FunctionalGroupDense)
    );
}

#[test]
fn a_fused_polyfunctional_system_does_not_trigger_dense_functionalization() {
    // Documented weak spot (rules::FG_WEIGHT_PER_DISTINCT_GROUP): glucose's
    // 6 hydroxyls sit on one connected ring and collapse to a single
    // identify_functional_groups cluster, identical in count to ethanol's
    // lone C-O group — confirmed empirically, not a regression to guard
    // against fixing here, just documenting the honest current behavior.
    let config = AnalysisConfig::default();
    let report =
        analyze_smiles("OC[C@H]1O[C@@H](O)[C@H](O)[C@@H](O)[C@@H]1O", &config).expect("glucose");
    assert!(
        !report
            .findings
            .iter()
            .any(|f| f.code == FindingCode::FunctionalGroupDense)
    );
}
