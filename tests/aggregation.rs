//! Cross-component aggregation contracts (AGENTS.md §20) — properties that
//! only exist once more than one difficulty-contributing component is
//! wired into `analyze`, so they can't live in a single component's test
//! file.

use rensei::AnalysisConfig;

#[test]
fn adding_size_topology_never_lowers_difficulty_below_ring_topology_alone() {
    // AGGREGATE_WEIGHT_RING_TOPOLOGY = 1.0 (full pass-through) plus a
    // non-negative additive size term is the property under test: a
    // molecule with real ring burden must not read as *easier* just
    // because a second component was added to the aggregation. This is
    // exactly the kind of named-constant arithmetic relationship that
    // stayed unverified until now (cf. the Indeterminate-threshold bug).
    let config = AnalysisConfig::default();
    let report = rensei::analyze_smiles("C1CC2CCC1C2", &config).expect("norbornane"); // bridged

    let ring_alone = report
        .components
        .ring_topology
        .as_ref()
        .expect("ring_topology always runs")
        .normalized
        .value();

    assert!(
        report.overall.difficulty.value() >= ring_alone,
        "overall difficulty {} fell below ring_topology alone {}",
        report.overall.difficulty.value(),
        ring_alone
    );
}

#[test]
fn heavier_molecules_are_not_prematurely_saturated_to_the_same_difficulty() {
    // Guards against the weighted sum silently clamping two distinguishable
    // molecules to the same ceiling value before `ProbabilityLikeScore`'s
    // 0.0..=1.0 clamp does its job — that would make them indistinguishable
    // even though they clearly differ in size/flexibility.
    let config = AnalysisConfig::default();
    let lighter = rensei::analyze_smiles("CCCCCCCCCCCCCC", &config).expect("14-carbon chain");
    let heavier = rensei::analyze_smiles("CCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCC", &config)
        .expect("40-carbon chain");

    let lighter_difficulty = lighter.overall.difficulty.value();
    let heavier_difficulty = heavier.overall.difficulty.value();

    assert!(
        heavier_difficulty > lighter_difficulty,
        "lighter={lighter_difficulty} heavier={heavier_difficulty}"
    );
    assert!(
        lighter_difficulty < 1.0,
        "lighter molecule already saturated at {lighter_difficulty}"
    );
}
