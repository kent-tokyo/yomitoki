//! Cross-component aggregation contracts (AGENTS.md §20) — properties that
//! only exist once more than one difficulty-contributing component is
//! wired into `analyze`, so they can't live in a single component's test
//! file.

use rensei::{AnalysisConfig, FindingCode};

#[test]
fn adding_a_component_never_lowers_difficulty_below_any_single_component_alone() {
    // AGGREGATE_WEIGHT_RING_TOPOLOGY = 1.0 (full pass-through) plus
    // non-negative additive size/stereo terms is the property under test:
    // a molecule must not read as *easier* just because more components
    // were wired into the aggregation. Uses a fixture with real ring,
    // size, and stereo burden simultaneously so all three are exercised,
    // not just ring_topology as in the original single-component version
    // of this test.
    let config = AnalysisConfig::default();
    let report =
        rensei::analyze_smiles("C1CC2CCC1C2C(N)C(O)C(Cl)C", &config).expect("valid SMILES");

    let components = &report.components;
    let each_alone = [
        ("ring_topology", &components.ring_topology),
        ("size_topology", &components.size_topology),
        ("stereochemical_burden", &components.stereochemical_burden),
    ];

    for (name, component) in each_alone {
        let normalized = component
            .as_ref()
            .unwrap_or_else(|| panic!("{name} always runs"))
            .normalized
            .value();
        assert!(
            report.overall.difficulty.value() >= normalized,
            "overall difficulty {} fell below {name} alone {normalized}",
            report.overall.difficulty.value(),
        );
    }
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
    let ring_wins = rensei::analyze_smiles("C1CC2CCC1C2C(N)C(O)C(Cl)C", &config)
        .expect("valid SMILES")
        .dominant_penalties()[0]
        .code;
    assert_eq!(ring_wins, FindingCode::RingBridgedComplexity);

    // 6 stereocenters (weight 0.72) vs. the same bridged ring (weight
    // 0.6): stereo burden outranks ring burden once enough centers pile
    // up. Documents that this is expected under the current weights, not
    // a bug.
    let stereo_wins = rensei::analyze_smiles("OC1CC2(C(N)C(O)C(Cl)C(N)C)CCC1C2", &config)
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
