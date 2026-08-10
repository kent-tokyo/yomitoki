//! `yomitoki` CLI (AGENTS.md §15).
//!
//! ```text
//! yomitoki analyze "<SMILES>" [--format human|json|jsonl] [--fragment-corpus <dir>]
//! yomitoki analyze --input <file> [--format human|json|jsonl] [--output <file>] [--fragment-corpus <dir>]
//! ```
//!
//! `jsonl` output uses the same `{"input", "report"|"error"}` shape in both
//! single-molecule and batch mode, so a downstream line-by-line parser sees
//! one schema regardless of which invocation form produced it.
//!
//! `--input` accepts a `.sdf` file (via `chematic::mol::SdfReader`) or a
//! plain SMILES-per-line file (via `chematic::mol::SmilesRecordReader`,
//! which also tolerates an optional whitespace-separated name column —
//! the standard `.smi` convention). Batch mode preserves input order and
//! never stops on one record's failure (AGENTS.md §15: "1分子の失敗で全処理を
//! 停止しないモードを設ける") — a failed record becomes an error entry, not a
//! skipped one, and the process exits non-zero only after every record has
//! been attempted.
//!
//! `--fragment-corpus <dir>` loads a `tools/build-fragment-corpus` output
//! directory (via `FragmentCorpus::load_dir`) and enables the
//! `fragment_precedent` component for this run — omitted,
//! `fragment_precedent` stays `None`, the same as every prior CLI release
//! (no corpus ships with yomitoki itself; AGENTS.md §5.4). Loaded once,
//! before any molecule is analyzed, not per-record.

use std::fs::File;
use std::io::{BufReader, Write};
use std::process::ExitCode;
use std::sync::Arc;

use serde::Serialize;
use yomitoki::{AnalysisConfig, FragmentCorpus, SynthesizabilityReport, analyze, analyze_smiles};

#[derive(Clone, Copy, PartialEq)]
enum Format {
    Human,
    Json,
    Jsonl,
}

struct Args {
    smiles: Option<String>,
    input: Option<String>,
    output: Option<String>,
    format: Format,
    fragment_corpus: Option<String>,
}

fn parse_args(mut args: impl Iterator<Item = String>) -> Result<Args, String> {
    let mut smiles = None;
    let mut input = None;
    let mut output = None;
    let mut format = None;
    let mut fragment_corpus = None;

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--input" => input = Some(args.next().ok_or("--input requires a value")?),
            "--output" => output = Some(args.next().ok_or("--output requires a value")?),
            "--fragment-corpus" => {
                fragment_corpus = Some(args.next().ok_or("--fragment-corpus requires a value")?)
            }
            "--format" => {
                let value = args.next().ok_or("--format requires a value")?;
                format = Some(match value.as_str() {
                    "human" => Format::Human,
                    "json" => Format::Json,
                    "jsonl" => Format::Jsonl,
                    other => {
                        return Err(format!(
                            "unknown format {other:?} (expected human|json|jsonl)"
                        ));
                    }
                });
            }
            other if !other.starts_with("--") && smiles.is_none() && input.is_none() => {
                smiles = Some(other.to_string());
            }
            other => return Err(format!("unrecognized argument {other:?}")),
        }
    }

    if smiles.is_some() && input.is_some() {
        return Err("provide either a SMILES argument or --input, not both".to_string());
    }
    if smiles.is_none() && input.is_none() {
        return Err("provide a SMILES argument or --input <file>".to_string());
    }

    Ok(Args {
        smiles,
        input,
        output,
        format: format.unwrap_or(Format::Human),
        fragment_corpus,
    })
}

const USAGE: &str = "\
Usage:
  yomitoki analyze \"<SMILES>\" [--format human|json|jsonl] [--fragment-corpus <dir>]
  yomitoki analyze --input <file> [--format human|json|jsonl] [--output <file>] [--fragment-corpus <dir>]

<file> may be a .sdf file or a SMILES-per-line file (optionally with a
whitespace-separated name column).

<dir> for --fragment-corpus is a tools/build-fragment-corpus output
directory (containing fragment_frequencies.json and manifest.json).
Enables the fragment_precedent component; omitted, it stays None.";

#[derive(Serialize)]
struct BatchItem<'a> {
    input: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    report: Option<SynthesizabilityReport>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}

fn main() -> ExitCode {
    let mut raw_args = std::env::args().skip(1);
    match raw_args.next().as_deref() {
        Some("analyze") => {}
        Some("--help") | Some("-h") | None => {
            println!("{USAGE}");
            return ExitCode::SUCCESS;
        }
        Some(other) => {
            eprintln!("unknown subcommand {other:?}\n\n{USAGE}");
            return ExitCode::from(2);
        }
    }

    let args = match parse_args(raw_args) {
        Ok(args) => args,
        Err(message) => {
            eprintln!("error: {message}\n\n{USAGE}");
            return ExitCode::from(2);
        }
    };

    let mut writer: Box<dyn Write> = match &args.output {
        Some(path) => match File::create(path) {
            Ok(file) => Box::new(file),
            Err(e) => {
                eprintln!("error: could not create output file {path:?}: {e}");
                return ExitCode::FAILURE;
            }
        },
        None => Box::new(std::io::stdout()),
    };

    let mut config = AnalysisConfig::default();
    if let Some(dir) = &args.fragment_corpus {
        match FragmentCorpus::load_dir(dir) {
            Ok(corpus) => config.fragment_model.corpus = Some(Arc::new(corpus)),
            Err(e) => {
                eprintln!("error: could not load --fragment-corpus {dir:?}: {e}");
                return ExitCode::FAILURE;
            }
        }
    }

    if let Some(smiles) = &args.smiles {
        return run_single(smiles, &config, args.format, writer.as_mut());
    }

    let path = args.input.as_deref().expect("checked in parse_args");
    run_batch(path, &config, args.format, writer.as_mut())
}

fn run_single(
    smiles: &str,
    config: &AnalysisConfig,
    format: Format,
    writer: &mut dyn Write,
) -> ExitCode {
    let report = match analyze_smiles(smiles, config) {
        Ok(report) => report,
        Err(e) => {
            eprintln!("error: {e}");
            return ExitCode::FAILURE;
        }
    };

    let result = match format {
        Format::Human => writeln!(writer, "{}", render_human(&report)),
        Format::Json => writeln!(
            writer,
            "{}",
            serde_json::to_string_pretty(&report).expect("report always serializes")
        ),
        // Wrapped in the same `BatchItem` shape as batch-mode jsonl output so
        // a downstream line-by-line parser sees one schema regardless of
        // whether the input was a single SMILES or an --input file.
        Format::Jsonl => {
            let item = BatchItem {
                input: smiles,
                report: Some(report),
                error: None,
            };
            writeln!(
                writer,
                "{}",
                serde_json::to_string(&item).expect("item always serializes")
            )
        }
    };
    if let Err(e) = result {
        eprintln!("error: failed to write output: {e}");
        return ExitCode::FAILURE;
    }
    ExitCode::SUCCESS
}

/// One (label, molecule-or-error) pair read from a batch input file.
type LabeledMolecule = (String, Result<chematic::core::Molecule, String>);

/// Reads a batch input file, kept in file order — `SdfReader`/
/// `SmilesRecordReader` are both streaming iterators, but analysis happens
/// after collecting so that a file-read error and an analysis error are
/// reported the same way (as a per-record entry, never as a silent skip).
fn read_batch_records(path: &str) -> Result<Vec<LabeledMolecule>, String> {
    if path.to_ascii_lowercase().ends_with(".sdf") {
        let content =
            std::fs::read_to_string(path).map_err(|e| format!("could not read {path:?}: {e}"))?;
        Ok(chematic::mol::SdfReader::new(&content)
            .enumerate()
            .map(|(i, result)| match result {
                Ok((mol, meta)) => {
                    let label = if meta.name.is_empty() {
                        format!("record {}", i + 1)
                    } else {
                        meta.name
                    };
                    (label, Ok(mol))
                }
                Err(e) => (format!("record {}", i + 1), Err(e.to_string())),
            })
            .collect())
    } else {
        let file = File::open(path).map_err(|e| format!("could not open {path:?}: {e}"))?;
        let reader = chematic::mol::SmilesRecordReader::new(
            BufReader::new(file),
            chematic::mol::SmilesReaderOptions::default(),
        );
        Ok(reader
            .enumerate()
            .map(|(i, result)| match result {
                Ok(record) => {
                    let label = if record.name.is_empty() {
                        format!("record {}", i + 1)
                    } else {
                        record.name
                    };
                    (label, Ok(record.mol))
                }
                Err(e) => (format!("record {}", i + 1), Err(e.to_string())),
            })
            .collect())
    }
}

fn run_batch(
    path: &str,
    config: &AnalysisConfig,
    format: Format,
    writer: &mut dyn Write,
) -> ExitCode {
    let records = match read_batch_records(path) {
        Ok(records) => records,
        Err(message) => {
            eprintln!("error: {message}");
            return ExitCode::FAILURE;
        }
    };

    let mut any_failed = false;
    let mut json_items: Vec<BatchItem> = Vec::with_capacity(records.len());

    for (label, molecule_result) in &records {
        let outcome = molecule_result
            .as_ref()
            .map_err(|e| e.clone())
            .and_then(|mol| analyze(mol, config).map_err(|e| e.to_string()));

        match &outcome {
            Ok(_) => {}
            Err(_) => any_failed = true,
        }

        match format {
            Format::Human => {
                let block = match &outcome {
                    Ok(report) => render_human(report),
                    Err(message) => format!("ERROR: {message}"),
                };
                if let Err(e) = writeln!(writer, "=== {label} ===\n{block}\n") {
                    eprintln!("error: failed to write output: {e}");
                    return ExitCode::FAILURE;
                }
            }
            Format::Jsonl => {
                let item = BatchItem {
                    input: label,
                    report: outcome.as_ref().ok().cloned(),
                    error: outcome.as_ref().err().cloned(),
                };
                let line = serde_json::to_string(&item).expect("item always serializes");
                if let Err(e) = writeln!(writer, "{line}") {
                    eprintln!("error: failed to write output: {e}");
                    return ExitCode::FAILURE;
                }
            }
            Format::Json => {
                json_items.push(BatchItem {
                    input: label,
                    report: outcome.as_ref().ok().cloned(),
                    error: outcome.as_ref().err().cloned(),
                });
            }
        }
    }

    if format == Format::Json {
        let rendered = serde_json::to_string_pretty(&json_items).expect("items always serialize");
        if let Err(e) = writeln!(writer, "{rendered}") {
            eprintln!("error: failed to write output: {e}");
            return ExitCode::FAILURE;
        }
    }

    if any_failed {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}

fn render_human(report: &SynthesizabilityReport) -> String {
    let mut out = format!(
        "Verdict: {:?}\nSynthesizability: {:.2}\nConfidence: {:.2}",
        report.overall.verdict,
        report.overall.synthesizability.value(),
        report.overall.confidence.value(),
    );
    if !report.dominant_penalties().is_empty() {
        out.push_str("\n\nDominant penalties:");
        for (i, penalty) in report.dominant_penalties().iter().enumerate() {
            out.push_str(&format!("\n{}. {}", i + 1, penalty.name));
        }
    }
    if !report.suggestions.is_empty() {
        out.push_str("\n\nSimplification suggestions (heuristic, not a guarantee):");
        for (i, suggestion) in report.suggestions.iter().enumerate() {
            out.push_str(&format!(
                "\n{}. {:?}: {}",
                i + 1,
                suggestion.code,
                suggestion.rationale
            ));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(strs: &[&str]) -> std::vec::IntoIter<String> {
        strs.iter()
            .map(|s| s.to_string())
            .collect::<Vec<_>>()
            .into_iter()
    }

    #[test]
    fn single_smiles_defaults_to_human_format() {
        let parsed = parse_args(args(&["CCO"])).expect("valid");
        assert_eq!(parsed.smiles.as_deref(), Some("CCO"));
        assert!(parsed.input.is_none());
        assert!(parsed.format == Format::Human);
    }

    #[test]
    fn format_flag_is_parsed() {
        let parsed = parse_args(args(&["CCO", "--format", "json"])).expect("valid");
        assert!(parsed.format == Format::Json);
    }

    #[test]
    fn unknown_format_is_rejected() {
        assert!(parse_args(args(&["CCO", "--format", "yaml"])).is_err());
    }

    #[test]
    fn input_and_smiles_together_is_rejected() {
        assert!(parse_args(args(&["CCO", "--input", "molecules.smi"])).is_err());
    }

    #[test]
    fn neither_smiles_nor_input_is_rejected() {
        assert!(parse_args(args(&["--format", "json"])).is_err());
    }

    #[test]
    fn missing_flag_value_is_rejected() {
        assert!(parse_args(args(&["CCO", "--format"])).is_err());
        assert!(parse_args(args(&["--input"])).is_err());
        assert!(parse_args(args(&["--output"])).is_err());
        assert!(parse_args(args(&["CCO", "--fragment-corpus"])).is_err());
    }

    #[test]
    fn fragment_corpus_flag_is_parsed_and_defaults_to_none() {
        let without = parse_args(args(&["CCO"])).expect("valid");
        assert!(without.fragment_corpus.is_none());

        let with = parse_args(args(&["CCO", "--fragment-corpus", "corpus_dir"])).expect("valid");
        assert_eq!(with.fragment_corpus.as_deref(), Some("corpus_dir"));
    }

    #[test]
    fn input_output_and_format_are_parsed_together() {
        let parsed = parse_args(args(&[
            "--input",
            "molecules.smi",
            "--format",
            "jsonl",
            "--output",
            "reports.jsonl",
        ]))
        .expect("valid");
        assert_eq!(parsed.input.as_deref(), Some("molecules.smi"));
        assert_eq!(parsed.output.as_deref(), Some("reports.jsonl"));
        assert!(parsed.format == Format::Jsonl);
    }

    #[test]
    fn render_human_matches_the_spec_shape() {
        let config = AnalysisConfig::default();
        let report = analyze_smiles("C1CC2CCC1C2", &config).expect("norbornane");
        let text = render_human(&report);
        assert!(text.starts_with("Verdict: "));
        assert!(text.contains("Synthesizability: "));
        assert!(text.contains("Confidence: "));
        assert!(text.contains("Dominant penalties:"));
        assert!(text.contains("Simplification suggestions"));
    }

    #[test]
    fn render_human_omits_dominant_penalties_and_suggestions_when_empty() {
        let config = AnalysisConfig::default();
        let report = analyze_smiles("CCO", &config).expect("ethanol");
        let text = render_human(&report);
        assert!(!text.contains("Dominant penalties"));
        assert!(!text.contains("Simplification suggestions"));
    }

    #[test]
    fn single_mode_jsonl_uses_the_same_wrapper_shape_as_batch_mode() {
        let config = AnalysisConfig::default();
        let mut buffer = Vec::new();
        let code = run_single("CCO", &config, Format::Jsonl, &mut buffer);
        assert!(code == ExitCode::SUCCESS);
        let line = String::from_utf8(buffer).expect("utf8");
        let value: serde_json::Value = serde_json::from_str(line.trim()).expect("valid json");
        assert_eq!(value["input"], "CCO");
        assert!(value.get("report").is_some());
        assert!(value.get("error").is_none());
    }
}
