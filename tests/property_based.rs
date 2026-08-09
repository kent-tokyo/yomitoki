//! Property-based invariants (AGENTS.md §14.4): across many
//! (molecule, config) combinations, `analyze` must never panic, never
//! produce NaN/Infinity, never leave a score outside `0.0..=1.0`, every
//! `Contribution` must stay within that same probability range, every
//! finding's atom indices must be within the source molecule's atom count,
//! and the same holds for every suggestion's confidence and target atoms.
//!
//! §14.4's own wording asks that "the contribution sum satisfies the
//! contract" — read here as "every individual contribution stays in
//! probability range," a narrower check than validating the sum against
//! the aggregate weights, since those weights (`AGGREGATE_WEIGHT_*` in
//! `rules.rs`) are `pub(crate)` and unreachable from an integration test.
//!
//! There's no general-purpose "generate an arbitrary valid molecule"
//! strategy available (chematic has no such generator, and writing a full
//! SMILES-grammar fuzzer is out of scope for this slice) — instead this
//! combines a small set of guaranteed-valid, parametric structural families
//! (linear chains, simple monocyclic rings) with a fixed corpus of
//! structurally distinct fixtures already used elsewhere in this test
//! suite (stereocenters, bridged/fused/spiro/macrocyclic rings, Brenk
//! alerts). The `Err(_) => return Ok(())` fallback below only guards
//! against a generator bug producing unparseable output; every branch is
//! expected to always parse.

use chematic::smiles::parse;
use proptest::prelude::*;
use rensei::{AnalysisConfig, Strictness, analyze};

fn linear_chain_smiles(n: usize) -> String {
    "C".repeat(n.max(1))
}

fn simple_ring_smiles(n: usize) -> String {
    format!("C1{}1", "C".repeat(n.saturating_sub(1)))
}

fn molecule_smiles_strategy() -> impl Strategy<Value = String> {
    prop_oneof![
        (1usize..80).prop_map(linear_chain_smiles),
        (3usize..30).prop_map(simple_ring_smiles),
        prop::sample::select(
            [
                "CCO",
                "c1ccccc1",
                "C1CC2CCC1C2",                      // bridged (norbornane)
                "CC(O)C(N)C(C)C(O)C(N)C",           // stereocenter-dense
                "C1CO1",                            // epoxide (Brenk alert)
                "CC(=O)Oc1ccccc1C(=O)O",            // aspirin (multiple Brenk alerts)
                "CC(=O)Nc1ccc(O)cc1",               // paracetamol
                "Cn1cnc2c1c(=O)n(c(=O)n2C)C",       // caffeine (fused heterocycle)
                "CC(=O)Cl",                         // acyl halide
                "C1CC1",                            // strained ring
                "CC#N",                             // nitrile
                "C[C@H](N)C(=O)O",                  // specified stereocenter
                "OC1CC2(C(N)C(O)C(Cl)C(N)C)CCC1C2", // bridged + several stereocenters
                "C1CCC2(CC1)CCCC2",                 // spiro
            ]
            .as_slice()
        )
        .prop_map(String::from),
    ]
}

fn strictness_strategy() -> impl Strategy<Value = Strictness> {
    prop_oneof![
        Just(Strictness::Lenient),
        Just(Strictness::Standard),
        Just(Strictness::Strict),
    ]
}

fn assert_probability_like(value: f64, label: &str) {
    assert!(value.is_finite(), "{label} is not finite: {value}");
    assert!(
        (0.0..=1.0).contains(&value),
        "{label} out of 0.0..=1.0 range: {value}"
    );
}

proptest! {
    #[test]
    fn analyze_never_panics_and_stays_in_contract(
        smiles in molecule_smiles_strategy(),
        strictness in strictness_strategy(),
        max_heavy_atoms in 1usize..200,
    ) {
        let Ok(mol) = parse(&smiles) else {
            return Ok(());
        };

        let mut config = AnalysisConfig::default();
        config.strictness = strictness;
        config.max_heavy_atoms = max_heavy_atoms;

        let report = analyze(&mol, &config).expect("a parsed molecule always yields Ok");

        assert_probability_like(report.overall.difficulty.value(), "difficulty");
        assert_probability_like(report.overall.synthesizability.value(), "synthesizability");
        assert_probability_like(report.overall.confidence.value(), "confidence");

        for contribution in &report.dominant_penalties {
            assert_probability_like(contribution.contribution.value(), "contribution");
        }

        for score in [
            &report.components.size_topology,
            &report.components.ring_topology,
            &report.components.stereochemical_burden,
            &report.components.functional_group_liability,
            &report.components.input_quality,
        ]
        .into_iter()
        .flatten()
        {
            // `raw` is deliberately unbounded (pre-normalization burden,
            // see rules.rs's `1 - exp(-raw / scale)` transform) — only
            // `normalized`/`confidence`/`contribution` are contractually
            // probability-like.
            prop_assert!(score.raw.is_finite(), "component raw is not finite: {}", score.raw);
            assert_probability_like(score.normalized.value(), "component normalized");
            assert_probability_like(score.confidence.value(), "component confidence");
            assert_probability_like(score.contribution.value(), "component contribution");
        }

        let atom_count = mol.atom_count() as u32;
        for finding in &report.findings {
            for atom in &finding.atoms {
                prop_assert!(
                    atom.0 < atom_count,
                    "finding {:?} references atom {} but molecule only has {atom_count} atoms",
                    finding.code,
                    atom.0
                );
            }
        }

        for suggestion in &report.suggestions {
            assert_probability_like(suggestion.confidence.value(), "suggestion confidence");
            for atom in &suggestion.target_atoms {
                prop_assert!(
                    atom.0 < atom_count,
                    "suggestion {:?} references atom {} but molecule only has {atom_count} atoms",
                    suggestion.code,
                    atom.0
                );
            }
        }
    }
}
