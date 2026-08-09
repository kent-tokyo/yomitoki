use yomitoki::{AnalysisConfig, analyze_smiles};

fn main() {
    let config = AnalysisConfig::default();

    for smiles in [
        "CCO",
        "c1ccccc1",
        "C1CC2CCC1C2",
        "CC(O)C(N)C(C)C(O)C(N)C",
        "C1CO1",
        "C1CCCCCCCC1",
    ] {
        let report = analyze_smiles(smiles, &config).expect("valid SMILES");

        println!("=== {smiles} ===");
        println!("Verdict: {:?}", report.overall.verdict);
        println!(
            "Synthesizability: {:.2}",
            report.overall.synthesizability.value()
        );
        println!("Confidence: {:.2}", report.overall.confidence.value());

        if !report.dominant_penalties().is_empty() {
            println!("Dominant penalties:");
            for (i, penalty) in report.dominant_penalties().iter().enumerate() {
                println!("{}. {}", i + 1, penalty.name);
            }
        }
        if !report.suggestions.is_empty() {
            println!("Simplification suggestions (heuristic, not a guarantee):");
            for (i, suggestion) in report.suggestions.iter().enumerate() {
                println!("{}. {:?}: {}", i + 1, suggestion.code, suggestion.rationale);
            }
        }
        println!();
    }
}
