//! Minimum comparison against SAscore (AGENTS.md §27 v0.1 completion
//! criterion: "SAscoreとの最低限の比較結果がある"). In-process only —
//! `chematic::chem::sa_score` (Ertl & Schuffenhauer 2009) is a Rust function
//! in a dependency yomitoki already has, not a competing Python
//! implementation, so this doesn't need the external `benchmarks/` script
//! setup AGENTS.md §13 describes for SYBA/SCScore/RAscore.
//!
//! Not a validation or accuracy claim — the two scores are not calibrated
//! against each other and measure different things (SAscore: fragment
//! frequency + complexity penalty, on RDKit's Morgan-fingerprint corpus;
//! yomitoki: structural burden, decomposed by component). This just makes
//! the comparison exist and shows where the two agree and disagree, and
//! why disagreement isn't automatically a yomitoki bug — see
//! `docs/architecture.md`'s "Comparison with SAscore" section.
//!
//! Scales run in opposite directions: SAscore is 1 (easy) .. 10 (hard);
//! yomitoki's `difficulty` is 0.0 (easy) .. 1.0 (hard). Values below are
//! printed raw, not rescaled onto a shared axis — rescaling would imply an
//! equivalence between the two that isn't established.

use yomitoki::{AnalysisConfig, analyze_smiles};

fn main() {
    let config = AnalysisConfig::default();

    // Same 14-fixture corpus as tests/property_based.rs's fixed-molecule
    // arm, reused rather than inventing a second one.
    let fixtures: &[(&str, &str)] = &[
        ("ethanol", "CCO"),
        ("benzene", "c1ccccc1"),
        ("norbornane (bridged)", "C1CC2CCC1C2"),
        ("stereocenter-dense fragment", "CC(O)C(N)C(C)C(O)C(N)C"),
        ("epoxide", "C1CO1"),
        ("aspirin", "CC(=O)Oc1ccccc1C(=O)O"),
        ("paracetamol", "CC(=O)Nc1ccc(O)cc1"),
        ("caffeine (fused heterocycle)", "Cn1cnc2c1c(=O)n(c(=O)n2C)C"),
        ("acyl halide", "CC(=O)Cl"),
        ("cyclopropane (strained)", "C1CC1"),
        ("nitrile", "CC#N"),
        ("alanine (specified stereocenter)", "C[C@H](N)C(=O)O"),
        (
            "bridged ring + several stereocenters",
            "OC1CC2(C(N)C(O)C(Cl)C(N)C)CCC1C2",
        ),
        ("spiro ring system", "C1CCC2(CC1)CCCC2"),
    ];

    println!(
        "{:38} {:>9} {:>18}  verdict",
        "molecule", "sa_score", "yomitoki_diff"
    );
    for (name, smiles) in fixtures {
        let mol = chematic::smiles::parse(smiles).expect("fixture SMILES must parse");
        let sa = chematic::chem::sa_score(&mol);
        let report = analyze_smiles(smiles, &config).expect("fixture SMILES must analyze");
        println!(
            "{:38} {:9.2} {:18.2}  {:?}",
            name,
            sa,
            report.overall.difficulty.value(),
            report.overall.verdict
        );
    }
}
