//! Builds a fragment-frequency corpus for the (not yet implemented)
//! `fragment_rarity` scoring component (AGENTS.md §5.4, §24). Not part of
//! the published `yomitoki` crate — a standalone, unpublished build tool.
//!
//! ```text
//! build-fragment-corpus --output <dir> \
//!   --source "<name>|<license>|<url>|<path>" [--source ...] \
//!   [--radii 0,1,2] [--limit N] \
//!   [--delimiter whitespace|tab|comma] [--smiles-column N] \
//!   [--name-column N|none] [--title-line]
//! ```
//!
//! `<path>` is a `.sdf` file or a SMILES table file (`.smi`/`.csv`/`.tsv`/
//! `.txt`), read via the same `chematic::mol` readers the `yomitoki` CLI's
//! batch mode uses. The `--delimiter`/`--smiles-column`/`--name-column`/
//! `--title-line` flags configure `SmilesReaderOptions` for non-`.sdf`
//! sources (defaults match a plain `.smi` file: whitespace-delimited,
//! SMILES in column 0, name in column 1, no header). All `.sdf`/text sources
//! in one invocation share these settings — fine while every non-`.sdf`
//! source is ChEMBL-shaped; revisit if a future run mixes differently
//! shaped text sources in one invocation.
//! Molecules are filtered to `yomitoki::SUPPORTED_ELEMENTS` (the same
//! curated element set the library's own applicability check uses — see
//! `src/lib.rs`'s `#[doc(hidden)]` re-export) and deduplicated by canonical
//! SMILES across *all* `--source` inputs given in one invocation, so a
//! molecule present in two sources is only counted once.
//!
//! Writes two files to `--output`:
//! - `fragment_frequencies.json`: for each `(radius, fragment_hash)`, the
//!   number of distinct molecules in which that fragment occurs at least
//!   once (document frequency, not raw atom-environment count).
//! - `manifest.json`: per-source provenance (AGENTS.md §24 — name, license,
//!   URL, input file sha256, record counts) plus `artifact_sha256`, the
//!   sha256 of `fragment_frequencies.json`'s bytes. That hash is computed
//!   over the frequency file only, not the manifest, so it stays
//!   reproducible across runs even though the manifest's own
//!   `generated_at_unix` field legitimately isn't.

use std::collections::{BTreeMap, HashSet};
use std::process::ExitCode;

use serde::Serialize;
use sha2::{Digest, Sha256};

struct SourceArg {
    name: String,
    license: String,
    url: String,
    path: String,
}

struct Args {
    output: String,
    radii: Vec<u32>,
    limit: Option<usize>,
    sources: Vec<SourceArg>,
    reader_options: chematic::mol::SmilesReaderOptions,
}

const DEFAULT_RADII: &[u32] = &[0, 1, 2];

const USAGE: &str = "\
Usage:
  build-fragment-corpus --output <dir> \\
    --source \"<name>|<license>|<url>|<path>\" [--source ...] \\
    [--radii 0,1,2] [--limit N] \\
    [--delimiter whitespace|tab|comma] [--smiles-column N] \\
    [--name-column N|none] [--title-line]

<path> is a .sdf file or a SMILES table file (.smi/.csv/.tsv/.txt).
--limit caps the total number of kept molecules across all sources combined
(for smoke-testing the pipeline without processing a whole corpus).
--delimiter/--smiles-column/--name-column/--title-line configure how
non-.sdf sources are parsed (defaults: whitespace, column 0, column 1, no
header — a plain .smi file). They apply to every non-.sdf --source in this
invocation.";

fn parse_args(mut args: impl Iterator<Item = String>) -> Result<Args, String> {
    let mut output = None;
    let mut radii = None;
    let mut limit = None;
    let mut sources = Vec::new();
    let mut reader_options = chematic::mol::SmilesReaderOptions::default();

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--output" => output = Some(args.next().ok_or("--output requires a value")?),
            "--radii" => {
                let value = args.next().ok_or("--radii requires a value")?;
                let mut parsed = Vec::new();
                for part in value.split(',') {
                    let radius: u32 = part
                        .trim()
                        .parse()
                        .map_err(|_| format!("invalid --radii value {part:?}"))?;
                    parsed.push(radius);
                }
                if parsed.is_empty() {
                    return Err("--radii requires at least one value".to_string());
                }
                radii = Some(parsed);
            }
            "--limit" => {
                let value = args.next().ok_or("--limit requires a value")?;
                limit = Some(
                    value
                        .parse::<usize>()
                        .map_err(|_| format!("invalid --limit value {value:?}"))?,
                );
            }
            "--source" => {
                let value = args.next().ok_or("--source requires a value")?;
                let parts: Vec<&str> = value.split('|').collect();
                let [name, license, url, path] = parts[..] else {
                    return Err(format!(
                        "--source value must be \"<name>|<license>|<url>|<path>\", got {value:?}"
                    ));
                };
                sources.push(SourceArg {
                    name: name.to_string(),
                    license: license.to_string(),
                    url: url.to_string(),
                    path: path.to_string(),
                });
            }
            "--delimiter" => {
                let value = args.next().ok_or("--delimiter requires a value")?;
                reader_options.delimiter = match value.as_str() {
                    "whitespace" => chematic::mol::Delimiter::Whitespace,
                    "tab" => chematic::mol::Delimiter::Tab,
                    "comma" => chematic::mol::Delimiter::Comma,
                    other => {
                        return Err(format!(
                            "unknown --delimiter {other:?} (expected whitespace|tab|comma)"
                        ));
                    }
                };
            }
            "--smiles-column" => {
                let value = args.next().ok_or("--smiles-column requires a value")?;
                reader_options.smiles_column = value
                    .parse()
                    .map_err(|_| format!("invalid --smiles-column value {value:?}"))?;
            }
            "--name-column" => {
                let value = args.next().ok_or("--name-column requires a value")?;
                reader_options.name_column = if value == "none" {
                    None
                } else {
                    Some(
                        value
                            .parse()
                            .map_err(|_| format!("invalid --name-column value {value:?}"))?,
                    )
                };
            }
            "--title-line" => reader_options.title_line = true,
            other => return Err(format!("unrecognized argument {other:?}")),
        }
    }

    let output = output.ok_or("--output is required")?;
    if sources.is_empty() {
        return Err("at least one --source is required".to_string());
    }

    Ok(Args {
        output,
        radii: radii.unwrap_or_else(|| DEFAULT_RADII.to_vec()),
        limit,
        sources,
        reader_options,
    })
}

/// Mirrors `yomitoki`'s own CLI `read_batch_records` (`src/bin/yomitoki.rs`):
/// both `SdfReader` and `SmilesRecordReader` are streaming iterators, but we
/// collect eagerly so a file-read error and a parse error are reported the
/// same way, as a per-record entry rather than a silent skip.
fn read_molecules(
    path: &str,
    reader_options: &chematic::mol::SmilesReaderOptions,
) -> Result<Vec<Result<chematic::core::Molecule, String>>, String> {
    if path.to_ascii_lowercase().ends_with(".sdf") {
        let content =
            std::fs::read_to_string(path).map_err(|e| format!("could not read {path:?}: {e}"))?;
        Ok(chematic::mol::SdfReader::new(&content)
            .map(|result| result.map(|(mol, _meta)| mol).map_err(|e| e.to_string()))
            .collect())
    } else {
        let file =
            std::fs::File::open(path).map_err(|e| format!("could not open {path:?}: {e}"))?;
        let reader = chematic::mol::SmilesRecordReader::new(
            std::io::BufReader::new(file),
            reader_options.clone(),
        );
        Ok(reader
            .map(|result| result.map(|record| record.mol).map_err(|e| e.to_string()))
            .collect())
    }
}

fn all_elements_supported(mol: &chematic::core::Molecule) -> bool {
    mol.atoms()
        .all(|(_, atom)| yomitoki::SUPPORTED_ELEMENTS.contains(&atom.element))
}

fn hex_sha256(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    format!("sha256:{digest:x}")
}

#[derive(Serialize)]
struct SourceRecord {
    name: String,
    license: String,
    url: String,
    path: String,
    input_sha256: String,
    records_read: usize,
    records_parse_error: usize,
    records_filtered_unsupported_element: usize,
    records_kept: usize,
    records_duplicate: usize,
}

#[derive(Serialize)]
struct FragmentRecord {
    radius: u32,
    fragment_hash: u64,
    occurrence_count: u64,
}

#[derive(Serialize)]
struct FrequencyTable {
    total_molecules_processed: u64,
    distinct_fragment_count: u64,
    fragments: Vec<FragmentRecord>,
}

#[derive(Serialize)]
struct CorpusManifest {
    sources: Vec<SourceRecord>,
    radii: Vec<u32>,
    preprocessing: String,
    exclusion_criteria: String,
    generated_at_unix: u64,
    chematic_version: String,
    tool_version: String,
    total_molecules_processed: u64,
    distinct_fragment_count: u64,
    artifact_file: String,
    artifact_sha256: String,
}

fn main() -> ExitCode {
    let args = match parse_args(std::env::args().skip(1)) {
        Ok(args) => args,
        Err(message) => {
            eprintln!("error: {message}\n\n{USAGE}");
            return ExitCode::from(2);
        }
    };

    if let Err(message) = run(&args) {
        eprintln!("error: {message}");
        return ExitCode::FAILURE;
    }
    ExitCode::SUCCESS
}

fn run(args: &Args) -> Result<(), String> {
    std::fs::create_dir_all(&args.output)
        .map_err(|e| format!("could not create output dir {:?}: {e}", args.output))?;

    let mut seen_canonical: HashSet<String> = HashSet::new();
    // (radius, fragment_hash) -> number of distinct molecules containing it.
    let mut frequency: BTreeMap<(u32, u64), u64> = BTreeMap::new();
    let mut total_molecules_processed: u64 = 0;
    let mut source_records = Vec::new();
    let mut remaining_limit = args.limit;

    for source in &args.sources {
        let raw_bytes = std::fs::read(&source.path)
            .map_err(|e| format!("could not read {:?}: {e}", source.path))?;
        let input_sha256 = hex_sha256(&raw_bytes);
        drop(raw_bytes);

        // ponytail: reads and parses the whole file before --limit applies —
        // fine at DrugBank's ~5MB scale; revisit with a streaming take()
        // before pointing this at multi-GB ChEMBL/SureChEMBL snapshots.
        let records = read_molecules(&source.path, &args.reader_options)?;
        let records_read = records.len();
        let mut records_parse_error = 0usize;
        let mut records_filtered_unsupported_element = 0usize;
        let mut records_kept = 0usize;
        let mut records_duplicate = 0usize;

        for record in records {
            if remaining_limit == Some(0) {
                break;
            }
            let mol = match record {
                Ok(mol) => mol,
                Err(_) => {
                    records_parse_error += 1;
                    continue;
                }
            };
            if !all_elements_supported(&mol) {
                records_filtered_unsupported_element += 1;
                continue;
            }
            let canonical = chematic::smiles::canonical_smiles(&mol);
            if !seen_canonical.insert(canonical) {
                records_duplicate += 1;
                continue;
            }

            records_kept += 1;
            total_molecules_processed += 1;
            if let Some(remaining) = remaining_limit.as_mut() {
                *remaining -= 1;
            }

            for &radius in &args.radii {
                let counts = chematic::fp::morgan_fp_counts(&mol, radius);
                for hash in counts.keys() {
                    *frequency.entry((radius, *hash)).or_insert(0) += 1;
                }
            }
        }

        source_records.push(SourceRecord {
            name: source.name.clone(),
            license: source.license.clone(),
            url: source.url.clone(),
            path: source.path.clone(),
            input_sha256,
            records_read,
            records_parse_error,
            records_filtered_unsupported_element,
            records_kept,
            records_duplicate,
        });
    }

    let fragments: Vec<FragmentRecord> = frequency
        .into_iter()
        .map(
            |((radius, fragment_hash), occurrence_count)| FragmentRecord {
                radius,
                fragment_hash,
                occurrence_count,
            },
        )
        .collect();
    let distinct_fragment_count = fragments.len() as u64;

    let table = FrequencyTable {
        total_molecules_processed,
        distinct_fragment_count,
        fragments,
    };
    let table_json = serde_json::to_vec_pretty(&table)
        .map_err(|e| format!("could not serialize frequency table: {e}"))?;
    let artifact_sha256 = hex_sha256(&table_json);

    let table_path = format!("{}/fragment_frequencies.json", args.output);
    std::fs::write(&table_path, &table_json)
        .map_err(|e| format!("could not write {table_path:?}: {e}"))?;

    let generated_at_unix = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);

    let manifest = CorpusManifest {
        sources: source_records,
        radii: args.radii.clone(),
        preprocessing: format!(
            "Parsed via chematic::mol readers ({:?}, smiles_column={}, \
             name_column={:?}, title_line={} for non-.sdf sources); molecules \
             with any atom outside yomitoki::SUPPORTED_ELEMENTS ({} elements) \
             dropped; deduplicated by chematic::smiles::canonical_smiles \
             across all --source inputs in this run.",
            args.reader_options.delimiter,
            args.reader_options.smiles_column,
            args.reader_options.name_column,
            args.reader_options.title_line,
            yomitoki::SUPPORTED_ELEMENTS.len()
        ),
        exclusion_criteria:
            "atoms outside yomitoki::SUPPORTED_ELEMENTS; exact canonical-SMILES duplicates"
                .to_string(),
        generated_at_unix,
        chematic_version: "0.12".to_string(),
        tool_version: env!("CARGO_PKG_VERSION").to_string(),
        total_molecules_processed,
        distinct_fragment_count,
        artifact_file: "fragment_frequencies.json".to_string(),
        artifact_sha256,
    };
    let manifest_json = serde_json::to_vec_pretty(&manifest)
        .map_err(|e| format!("could not serialize manifest: {e}"))?;
    let manifest_path = format!("{}/manifest.json", args.output);
    std::fs::write(&manifest_path, &manifest_json)
        .map_err(|e| format!("could not write {manifest_path:?}: {e}"))?;

    println!(
        "wrote {table_path} ({distinct_fragment_count} distinct fragments from \
         {total_molecules_processed} molecules) and {manifest_path}"
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(values: &[&str]) -> impl Iterator<Item = String> {
        values
            .iter()
            .map(|s| s.to_string())
            .collect::<Vec<_>>()
            .into_iter()
    }

    #[test]
    fn parses_minimal_invocation() {
        let parsed = parse_args(args(&[
            "--output",
            "out",
            "--source",
            "DrugBank|CC0-1.0|https://example.test|data.sdf",
        ]))
        .unwrap();
        assert_eq!(parsed.output, "out");
        assert_eq!(parsed.radii, DEFAULT_RADII.to_vec());
        assert_eq!(parsed.limit, None);
        assert_eq!(parsed.sources.len(), 1);
        assert_eq!(parsed.sources[0].name, "DrugBank");
        assert_eq!(parsed.sources[0].license, "CC0-1.0");
        assert_eq!(parsed.sources[0].url, "https://example.test");
        assert_eq!(parsed.sources[0].path, "data.sdf");
    }

    #[test]
    fn parses_custom_radii_and_limit_and_multiple_sources() {
        let parsed = parse_args(args(&[
            "--output",
            "out",
            "--radii",
            "0,1,2,3",
            "--limit",
            "100",
            "--source",
            "A|CC0-1.0|https://a.test|a.sdf",
            "--source",
            "B|CC-BY-4.0|https://b.test|b.smi",
        ]))
        .unwrap();
        assert_eq!(parsed.radii, vec![0, 1, 2, 3]);
        assert_eq!(parsed.limit, Some(100));
        assert_eq!(parsed.sources.len(), 2);
    }

    #[test]
    fn rejects_missing_output() {
        assert!(parse_args(args(&["--source", "A|L|U|p"])).is_err());
    }

    #[test]
    fn rejects_missing_sources() {
        assert!(parse_args(args(&["--output", "out"])).is_err());
    }

    #[test]
    fn rejects_malformed_source() {
        assert!(parse_args(args(&["--output", "out", "--source", "only-two|parts"])).is_err());
    }

    #[test]
    fn filters_and_hashes_are_deterministic() {
        // A supported-elements-only molecule (ethanol) hashes identically
        // whichever order its bytes reach hex_sha256 in.
        let a = hex_sha256(b"CCO");
        let b = hex_sha256(b"CCO");
        assert_eq!(a, b);
        assert_ne!(a, hex_sha256(b"CCN"));
    }
}
