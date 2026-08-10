//! Diagnostic tool: look up a SMILES's fragments in a built corpus and
//! report how well-covered they are. Not part of the `fragment_rarity`
//! scoring component — this exists to answer one question before that
//! component is designed: does a given corpus actually distinguish common,
//! precedented fragments from rare ones?
//!
//! ```text
//! query --corpus <dir> "<SMILES>" [--radii 0,1,2]
//! ```

use std::collections::HashMap;
use std::process::ExitCode;

use serde::Deserialize;

#[derive(Deserialize)]
struct FragmentRecord {
    radius: u32,
    fragment_hash: u64,
    occurrence_count: u64,
}

#[derive(Deserialize)]
struct FrequencyTable {
    total_molecules_processed: u64,
    fragments: Vec<FragmentRecord>,
}

fn main() -> ExitCode {
    let mut args = std::env::args().skip(1);
    let mut corpus_dir = None;
    let mut smiles = None;
    let mut radii = vec![0u32, 1, 2];

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--corpus" => corpus_dir = args.next(),
            "--radii" => {
                if let Some(value) = args.next() {
                    radii = value
                        .split(',')
                        .filter_map(|s| s.trim().parse().ok())
                        .collect();
                }
            }
            other => smiles = Some(other.to_string()),
        }
    }

    let (Some(corpus_dir), Some(smiles)) = (corpus_dir, smiles) else {
        eprintln!("usage: query --corpus <dir> \"<SMILES>\" [--radii 0,1,2]");
        return ExitCode::from(2);
    };

    let table_path = format!("{corpus_dir}/fragment_frequencies.json");
    let table_bytes = match std::fs::read(&table_path) {
        Ok(bytes) => bytes,
        Err(e) => {
            eprintln!("error: could not read {table_path:?}: {e}");
            return ExitCode::FAILURE;
        }
    };
    let table: FrequencyTable = match serde_json::from_slice(&table_bytes) {
        Ok(table) => table,
        Err(e) => {
            eprintln!("error: could not parse {table_path:?}: {e}");
            return ExitCode::FAILURE;
        }
    };
    let mut frequency: HashMap<(u32, u64), u64> = HashMap::new();
    for record in &table.fragments {
        frequency.insert(
            (record.radius, record.fragment_hash),
            record.occurrence_count,
        );
    }

    let mol = match chematic::smiles::parse(&smiles) {
        Ok(mol) => mol,
        Err(e) => {
            eprintln!("error: could not parse SMILES {smiles:?}: {e}");
            return ExitCode::FAILURE;
        }
    };

    println!(
        "corpus: {} molecules processed",
        table.total_molecules_processed
    );
    println!("query: {smiles}");
    for &radius in &radii {
        let counts = chematic::fp::morgan_fp_counts(&mol, radius);
        let total = counts.len();
        let mut found = 0usize;
        let mut min_df = f64::INFINITY;
        let mut max_df = 0.0f64;
        let mut sum_df = 0.0f64;
        for hash in counts.keys() {
            if let Some(&occurrence) = frequency.get(&(radius, *hash)) {
                found += 1;
                let df = occurrence as f64 / table.total_molecules_processed as f64;
                min_df = min_df.min(df);
                max_df = max_df.max(df);
                sum_df += df;
            }
        }
        println!(
            "  radius {radius}: {found}/{total} fragments found in corpus{}",
            if found > 0 {
                format!(
                    " (document frequency: min {:.6}, max {:.6}, mean {:.6})",
                    min_df,
                    max_df,
                    sum_df / found as f64
                )
            } else {
                String::new()
            }
        );
    }
    ExitCode::SUCCESS
}
