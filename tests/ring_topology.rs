//! Ring topology component behavior. Fixtures were confirmed against
//! chematic's actual ring-family classification before being used here
//! (see commit history / architecture notes) — not assumed from SMILES
//! alone.

use rensei::{AnalysisConfig, FindingCode, Verdict, analyze_smiles};

fn ring_difficulty(smiles: &str) -> f64 {
    let config = AnalysisConfig::default();
    let report = analyze_smiles(smiles, &config).expect("valid SMILES");
    report
        .components
        .ring_topology
        .expect("ring_topology always runs")
        .normalized
        .value()
}

#[test]
fn acyclic_molecule_has_zero_ring_burden() {
    assert_eq!(ring_difficulty("CCO"), 0.0);
}

#[test]
fn bridged_bicyclic_exceeds_monocyclic_analog() {
    // AGENTS.md §14.1: "bridged bicyclic molecules exceed monocyclic
    // analogs". Norbornane (bridged, 7 atoms) vs. cyclohexane (simple, 6
    // atoms) — both small saturated all-carbon rings, differing only in
    // topology.
    let bridged = ring_difficulty("C1CC2CCC1C2"); // norbornane
    let monocyclic = ring_difficulty("C1CCCCC1"); // cyclohexane
    assert!(
        bridged > monocyclic,
        "bridged={bridged} monocyclic={monocyclic}"
    );
}

#[test]
fn additional_isomorphic_ring_does_not_reduce_burden() {
    // AGENTS.md §14.1 metamorphic property, applied to ring topology:
    // adding ring complexity must never *decrease* the score.
    let simple = ring_difficulty("c1ccccc1"); // benzene
    let fused = ring_difficulty("C1CC2CCCCC2C1"); // fused bicyclic, 9 atoms
    assert!(fused >= simple, "fused={fused} simple={simple}");
}

#[test]
fn spiro_ring_system_is_flagged() {
    let config = AnalysisConfig::default();
    let report = analyze_smiles("C1CCC2(CC1)CCCC2", &config).expect("valid SMILES");
    assert!(
        report
            .findings
            .iter()
            .any(|f| f.code == FindingCode::RingSpiro)
    );
}

#[test]
fn bridged_ring_system_is_flagged() {
    let config = AnalysisConfig::default();
    let report = analyze_smiles("C1CC2CCC1C2", &config).expect("valid SMILES");
    assert!(
        report
            .findings
            .iter()
            .any(|f| f.code == FindingCode::RingBridgedComplexity)
    );
    assert_eq!(report.overall.verdict, Verdict::ModeratelyAccessible);
}

#[test]
fn macrocycle_at_threshold_is_flagged() {
    let config = AnalysisConfig::default();
    let report = analyze_smiles("C1CCCCCCCC1", &config).expect("valid SMILES"); // 9-membered ring
    assert!(
        report
            .findings
            .iter()
            .any(|f| f.code == FindingCode::RingMacrocycle)
    );
}

#[test]
fn dominant_penalties_are_ranked_by_actual_weight_not_evidence_magnitude() {
    // Bridged bicyclic system whose larger ring is also macrocycle-sized
    // (5- and 10-membered rings, bridged). Produces both a
    // RingBridgedComplexity finding (weight 0.6) and a RingMacrocycle
    // finding (weight 0.25, but with evidence.value = 10.0 — the ring
    // size). A naive sort by evidence magnitude would rank macrocycle
    // first; ranking must follow the actual contribution weight instead.
    //
    // Checks relative order (position of A before B), not an exact/closed
    // finding list — this fixture's bridgehead atoms also happen to be
    // stereocenters, so stereochemical_burden legitimately contributes a
    // third, lower-weight finding here too; a closed-list assertion would
    // make this test spuriously fragile to other components' correct
    // behavior on the same fixture.
    let config = AnalysisConfig::default();
    let report = analyze_smiles("C1CC2CCCCCCCC1C2", &config).expect("valid SMILES");

    let codes: Vec<FindingCode> = report.dominant_penalties().iter().map(|c| c.code).collect();
    let bridged_pos = codes
        .iter()
        .position(|c| *c == FindingCode::RingBridgedComplexity)
        .expect("RingBridgedComplexity present");
    let macrocycle_pos = codes
        .iter()
        .position(|c| *c == FindingCode::RingMacrocycle)
        .expect("RingMacrocycle present");
    assert!(
        bridged_pos < macrocycle_pos,
        "dominant_penalties order: {codes:?}"
    );

    let contributions: Vec<f64> = report
        .dominant_penalties()
        .iter()
        .map(|c| c.contribution.value())
        .collect();
    assert!(
        contributions[bridged_pos] > contributions[macrocycle_pos],
        "bridged contribution {} should exceed macrocycle contribution {}",
        contributions[bridged_pos],
        contributions[macrocycle_pos]
    );
}

#[test]
fn ring_below_macrocycle_threshold_is_not_flagged() {
    let config = AnalysisConfig::default();
    let report = analyze_smiles("c1ccccc1", &config).expect("valid SMILES"); // 6-membered
    assert!(
        !report
            .findings
            .iter()
            .any(|f| f.code == FindingCode::RingMacrocycle)
    );
}
