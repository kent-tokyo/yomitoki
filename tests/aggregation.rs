//! Cross-component aggregation contracts (AGENTS.md §20) — properties that
//! only exist once more than one difficulty-contributing component is
//! wired into `analyze`, so they can't live in a single component's test
//! file.

use yomitoki::{AnalysisConfig, FindingCode};

#[test]
fn ring_topology_is_a_full_pass_through_floor_for_difficulty() {
    // AGGREGATE_WEIGHT_RING_TOPOLOGY = 1.0 is the *only* aggregation weight
    // that's a full pass-through (see rules.rs) — every other component
    // contributes at a fractional weight (0.4/0.5/0.4), so
    // `difficulty >= component.normalized` is only guaranteed to hold for
    // ring_topology specifically, not for size/stereo/functional-group
    // liability in general.
    //
    // An earlier version of this test asserted that inequality for all
    // four components — false in general: a plain 40-carbon alkane has
    // size_topology.normalized ~0.52 but difficulty ~0.21, since
    // AGGREGATE_WEIGHT_SIZE_TOPOLOGY (0.4) dilutes it. It only passed
    // because its one fixture happened to have dominant ring burden
    // masking that. Caught by an advisor review, not by this test.
    let config = AnalysisConfig::default();
    let report =
        yomitoki::analyze_smiles("C1CC2CCC1C2C(N)C(O)C(Cl)C", &config).expect("valid SMILES");

    let ring_alone = report
        .components
        .ring_topology
        .as_ref()
        .expect("ring_topology always runs")
        .normalized
        .value();
    assert!(
        report.overall.difficulty.value() >= ring_alone,
        "overall difficulty {} fell below ring_topology alone {ring_alone}",
        report.overall.difficulty.value(),
    );
}

#[test]
fn adding_a_functional_group_liability_never_lowers_difficulty() {
    // Metamorphic property (AGENTS.md §14.3 style): a molecule that gains a
    // Brenk-alert-triggering feature, with the rest of its structure held
    // equal, must not read as *easier*. Ethane (no alerts) vs. acetonitrile
    // (CC#N, triggers the `nitrile` alert) isolates functional_group_
    // liability's contribution — both have zero ring and stereo burden,
    // and size_topology's own contribution barely moves between them
    // (confirmed via probe: 0.0090 vs 0.0122), so functional_group_
    // liability (0.0 vs 0.0769) is what actually drives the difference.
    let config = AnalysisConfig::default();
    let plain = yomitoki::analyze_smiles("CC", &config).expect("ethane");
    let with_fg_alert = yomitoki::analyze_smiles("CC#N", &config).expect("acetonitrile");

    assert!(
        with_fg_alert.overall.difficulty.value() >= plain.overall.difficulty.value(),
        "plain={} with_alert={}",
        plain.overall.difficulty.value(),
        with_fg_alert.overall.difficulty.value()
    );
}

#[test]
fn dominant_penalties_rank_across_components_by_contribution_not_by_component_identity() {
    // Pins down a real cross-component ranking outcome so a change to any
    // component's weight constants forces a conscious look at this test
    // rather than silently reordering which component "wins" — see
    // tasks/lessons.md for why this needed its own test instead of relying
    // on ring_topology's own (single-component) ranking test.
    let config = AnalysisConfig::default();

    // 2 stereocenters (weight 0.24) vs. one bridged ring (weight 0.6):
    // ring wins under the current STEREO_WEIGHT_PER_CENTER.
    let ring_wins = yomitoki::analyze_smiles("C1CC2CCC1C2C(N)C(O)C(Cl)C", &config)
        .expect("valid SMILES")
        .dominant_penalties()[0]
        .code;
    assert_eq!(ring_wins, FindingCode::RingBridgedComplexity);

    // 6 stereocenters (weight 0.72) vs. the same bridged ring (weight
    // 0.6): stereo burden outranks ring burden once enough centers pile
    // up. Documents that this is expected under the current weights, not
    // a bug.
    let stereo_wins = yomitoki::analyze_smiles("OC1CC2(C(N)C(O)C(Cl)C(N)C)CCC1C2", &config)
        .expect("valid SMILES")
        .dominant_penalties()[0]
        .code;
    assert_eq!(stereo_wins, FindingCode::StereoCenterCount);
}

#[test]
fn heavier_molecules_are_not_prematurely_saturated_to_the_same_difficulty() {
    // Guards against the weighted sum silently clamping two distinguishable
    // molecules to the same ceiling value before `ProbabilityLikeScore`'s
    // 0.0..=1.0 clamp does its job — that would make them indistinguishable
    // even though they clearly differ in size/flexibility.
    let config = AnalysisConfig::default();
    let lighter = yomitoki::analyze_smiles("CCCCCCCCCCCCCC", &config).expect("14-carbon chain");
    let heavier = yomitoki::analyze_smiles("CCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCC", &config)
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
