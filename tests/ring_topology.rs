//! Ring topology component behavior. Fixtures were confirmed against
//! chematic's actual ring-family classification before being used here
//! (see commit history / architecture notes) — not assumed from SMILES
//! alone.

use yomitoki::{AnalysisConfig, FindingCode, Verdict, analyze_smiles};

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
fn independent_rings_score_sublinearly_but_never_less_with_more_of_them() {
    // Three separate, ordinary (unfused) benzene rings connected by
    // single bonds -- multiplicity, not fusion. L2 aggregation (round 22
    // part 10, see ring_topology.rs's `family_burdens` comment) must
    // still grow monotonically with each additional independent ring
    // (never decrease -- same invariant `additional_isomorphic_ring_does_
    // not_reduce_burden` checks for fusion) but sublinearly: each
    // additional ordinary ring adds less than the previous one did.
    // Plain summing (L1, the prior formula) violated the second half of
    // this -- MPScore's mechanism check found chemist-easy molecules in
    // the complex-ring stratum have systematically MORE separate ring
    // families than chemist-hard ones at the same total ring count (e.g.
    // mean 1.11 vs. 3.16 families at 5 total rings), i.e. real molecules
    // where additional ordinary rings are cheap, not linearly costly.
    let one = ring_difficulty("c1ccccc1"); // benzene
    let two = ring_difficulty("c1ccc(-c2ccccc2)cc1"); // biphenyl
    let three = ring_difficulty("c1ccc(-c2ccc(-c3ccccc3)cc2)cc1"); // p-terphenyl

    assert!(two > one, "two={two} one={one}");
    assert!(three > two, "three={three} two={two}");
    assert!(
        (two - one) > (three - two),
        "marginal burden should shrink with each additional ring: (two-one)={} (three-two)={}",
        two - one,
        three - two
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

#[test]
fn single_ring_family_burden_is_exact_and_unaffected_by_l2_aggregation() {
    // L2 aggregation (round 22 part 10/11: raw = sqrt(Σ f_i²) across ring
    // families) is a mathematical no-op for any molecule with exactly one
    // ring family, since sqrt(x²) = x for x ≥ 0 -- but that's exactly the
    // kind of invariant a future refactor could silently break (e.g. by
    // squaring without the sqrt, or applying the aggregation before
    // filtering). Locks in the precise values for three single-family
    // fixtures spanning two kinds, computed from the known formula
    // (RING_WEIGHT_SIMPLE=0.15 / RING_WEIGHT_BRIDGED=0.6,
    // RING_BURDEN_SCALE=1.5, normalized = 1 - exp(-raw/scale)) and
    // cross-checked against the crate's own output.
    let eps = 1e-9;

    let cyclohexane = ring_difficulty("C1CCCCC1"); // Simple, raw=0.15
    assert!(
        (cyclohexane - 0.095_162_581_964_040).abs() < eps,
        "cyclohexane={cyclohexane}"
    );

    let norbornane = ring_difficulty("C1CC2CCC1C2"); // Bridged, raw=0.6
    assert!(
        (norbornane - 0.329_679_953_964_361).abs() < eps,
        "norbornane={norbornane}"
    );

    // Adamantane: a substantially more complex cage than norbornane, but
    // still classified as one Bridged ring family (chematic's
    // RingSystemKind has no further gradation within "Bridged") --
    // scores identically to norbornane both before and after this
    // change. Not something L2 introduced; asserted here so a future
    // change to either the aggregation or the per-kind weight doesn't
    // silently start conflating "single complex family" with "L2 dilutes
    // it" when the real (pre-existing, separate) limitation is that
    // `Bridged`'s own weight isn't extent-sensitive.
    let adamantane = ring_difficulty("C1C2CC3CC1CC(C2)C3"); // Bridged, raw=0.6
    assert!(
        (adamantane - norbornane).abs() < eps,
        "adamantane={adamantane} norbornane={norbornane}"
    );
}

#[test]
fn bridged_cage_finding_and_signal_survive_alongside_an_unrelated_ring() {
    // The instruction's explicit fallback check, as a permanent
    // regression test: a genuinely complex (Bridged) ring family must
    // keep firing its finding, and its *own* per-finding contribution
    // (what dominant_penalties ranks by) must stay exactly its true
    // weight, even when L2-aggregated alongside an unrelated, ordinary
    // ring family in the same molecule.
    let config = AnalysisConfig::default();
    let report = analyze_smiles("c1ccc(cc1)C1CC2CCC1C2", &config).expect("valid SMILES"); // phenyl-norbornane

    assert!(
        report
            .findings
            .iter()
            .any(|f| f.code == FindingCode::RingBridgedComplexity),
        "RingBridgedComplexity must still fire alongside an unrelated ring"
    );
    let bridged_contribution = report
        .dominant_penalties()
        .iter()
        .find(|c| c.code == FindingCode::RingBridgedComplexity)
        .expect("RingBridgedComplexity present in dominant_penalties")
        .contribution
        .value();
    assert!(
        (bridged_contribution - 0.6).abs() < 1e-9,
        "bridged family's own contribution must stay exactly its true weight (0.6), \
         not some L2-diluted fraction: got {bridged_contribution}"
    );
}

#[test]
fn aggregate_ring_contribution_is_l2_not_l1_while_per_finding_contributions_stay_true() {
    // "Explanation consistency": the component-level aggregate
    // (`ring_topology.contribution`, what feeds `overall.difficulty`)
    // must reflect the new L2 formula, while each individual finding's
    // own `Contribution` (what `dominant_penalties` ranks by) keeps
    // reporting its family's real, un-aggregated weight -- these are
    // deliberately different numbers for different purposes, and a
    // regression that collapsed them together (e.g. by re-deriving the
    // aggregate from summing dominant_penalties) would misreport why a
    // molecule scored the way it did.
    let config = AnalysisConfig::default();
    let report = analyze_smiles("c1ccc(cc1)C1CC2CCC1C2", &config).expect("valid SMILES"); // phenyl-norbornane, families: Simple(0.15) + Bridged(0.6)

    let rt = report
        .components
        .ring_topology
        .as_ref()
        .expect("ring_topology always runs");

    // L2 aggregate: sqrt(0.15² + 0.6²) = sqrt(0.3825) ≈ 0.618465843842649
    // -> normalized ≈ 0.337881385134783.
    let expected_l2_normalized = 0.337_881_385_134_783;
    assert!(
        (rt.normalized.value() - expected_l2_normalized).abs() < 1e-9,
        "aggregate ring_topology.normalized={}, expected L2 value={expected_l2_normalized}",
        rt.normalized.value()
    );

    // What plain L1 summing (the pre-round-22-part-10 formula) would have
    // given for the same two families: 1 - exp(-(0.15+0.6)/1.5) ≈
    // 0.393469340287367. The real aggregate must be strictly lower --
    // this is the whole point of the change, checked as an inequality
    // (not just a point value) so the test also fails loudly if a future
    // edit accidentally reverts to L1.
    let l1_alternative_normalized = 0.393_469_340_287_367;
    assert!(
        rt.normalized.value() < l1_alternative_normalized,
        "L2 aggregate ({}) must be strictly less than what L1 summing would give ({l1_alternative_normalized})",
        rt.normalized.value()
    );

    // The Bridged finding's own per-finding contribution is neither of
    // the above -- it's that family's true, un-aggregated weight.
    let bridged_contribution = report
        .dominant_penalties()
        .iter()
        .find(|c| c.code == FindingCode::RingBridgedComplexity)
        .expect("RingBridgedComplexity present")
        .contribution
        .value();
    assert!((bridged_contribution - 0.6).abs() < 1e-9);
    assert!(
        (bridged_contribution - rt.normalized.value()).abs() > 1e-3,
        "per-finding contribution and the component aggregate must be visibly different numbers"
    );
}
