use rensei::{AnalysisConfig, analyze_smiles};

fn main() {
    let config = AnalysisConfig::default();

    for smiles in ["CCO", "c1ccccc1", "C1CC2CCC1C2"] {
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
        println!();
    }
}
