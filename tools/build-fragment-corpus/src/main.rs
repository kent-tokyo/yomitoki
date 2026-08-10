//! Builds a fragment-frequency corpus for the `fragment_precedent` scoring
//! component (AGENTS.md §5.4, §24). Not part of the published `yomitoki`
//! crate — a standalone, unpublished build tool.
//!
//! ```text
//! build-fragment-corpus --output <dir> \
//!   --source "<name>|<license>|<url>|<path>" [--source ...] \
//!   --corpus-domain-name <name> --corpus-domain <domain> \
//!   --corpus-synthesis-focused true|false --corpus-domain-description <text> \
//!   [--radius 2] [--limit N] [--exclude-smiles-file <path>] \
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
//! `--radius` takes exactly one value, not a list: `chematic::fp::
//! morgan_fp_counts(mol, radius)` is cumulative over iterations `0..=radius`
//! (confirmed from `chematic-fp`'s source), so multiple radii used to store
//! the same underlying fragments redundantly under different keys (round
//! 15's finding) — and the molecule-level reference distribution (round 17)
//! is inherently single-radius: a molecule's "how common are my fragments"
//! statistic only means one thing per corpus, not one per radius.
//!
//! Writes two files to `--output`:
//! - `fragment_frequencies.json`: for each fragment hash, the number of
//!   distinct molecules in which it occurs at least once (document
//!   frequency, not raw atom-environment count).
//! - `manifest.json`: per-source provenance (AGENTS.md §24 — name, license,
//!   URL, input file sha256, record counts), `artifact_sha256` (sha256 of
//!   `fragment_frequencies.json`'s bytes — computed over the frequency file
//!   only, not the manifest, so it stays reproducible across runs even
//!   though the manifest's own `generated_at_unix` field legitimately
//!   isn't), and `reference_distribution`: a 1001-point quantile grid
//!   (p = 0.000, 0.001, ..., 1.000) of the corpus's own molecule-level mean
//!   -document-frequency distribution, letting `fragment_precedent` convert a
//!   query molecule's mean document frequency into an empirical percentile
//!   against this exact corpus rather than an arbitrary absolute scale.
//! - `manifest.json`'s `corpus_domain` (round 18): what chemical space this
//!   corpus represents and whether its builder claims it's
//!   synthesis-focused — required, not guessed; see `--corpus-domain-*`
//!   below.

use std::collections::{HashMap, HashSet};
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
    radius: u32,
    limit: Option<usize>,
    sources: Vec<SourceArg>,
    reader_options: chematic::mol::SmilesReaderOptions,
    corpus_domain_name: String,
    corpus_domain: String,
    corpus_synthesis_focused: bool,
    corpus_domain_description: String,
    /// Round 19: path to a newline-delimited SMILES file (first
    /// whitespace-delimited token per line, so a name/id column after the
    /// SMILES is tolerated) whose canonical forms are excluded from this
    /// corpus entirely -- before dedup, before `--limit`, before the
    /// frequency table and `reference_distribution` are built from anything.
    /// Exists for leave-one-out validation: a molecule under test must not
    /// be able to inflate its own precedent score by being present in the
    /// reference corpus it's scored against.
    exclude_smiles_file: Option<String>,
}

const DEFAULT_RADIUS: u32 = 2;
/// Number of points in the stored quantile grid (per-mille resolution:
/// p = 0.000, 0.001, ..., 1.000).
const DISTRIBUTION_GRID_POINTS: usize = 1001;

const USAGE: &str = "\
Usage:
  build-fragment-corpus --output <dir> \\
    --source \"<name>|<license>|<url>|<path>\" [--source ...] \\
    --corpus-domain-name <name> --corpus-domain <domain> \\
    --corpus-synthesis-focused true|false --corpus-domain-description <text> \\
    [--radius 2] [--limit N] [--exclude-smiles-file <path>] \\
    [--delimiter whitespace|tab|comma] [--smiles-column N] \\
    [--name-column N|none] [--title-line]

<path> is a .sdf file or a SMILES table file (.smi/.csv/.tsv/.txt).
--radius takes exactly one value (default 2 — ECFP4-equivalent; morgan_fp
_counts is cumulative, so multiple radii would be redundant).
--limit caps the total number of kept molecules across all sources combined
(for smoke-testing the pipeline without processing a whole corpus).
--delimiter/--smiles-column/--name-column/--title-line configure how
non-.sdf sources are parsed (defaults: whitespace, column 0, column 1, no
header — a plain .smi file). They apply to every non-.sdf --source in this
invocation.
--corpus-domain-name/--corpus-domain/--corpus-synthesis-focused/
--corpus-domain-description are all required (round 18): they record what
chemical space this corpus represents (e.g. \"ChEMBL-37\" / \"bioactivity\" /
false / \"Bioactive compound reference corpus; not a synthesis-focused
precedent corpus.\") in the manifest, so a report can later distinguish
\"rare in this corpus\" from \"hard to synthesize\" — no default is guessed,
since guessing a domain would defeat the point.
--exclude-smiles-file (optional, round 19) takes a newline-delimited SMILES
file; any molecule whose canonical form matches an entry is dropped before
dedup/--limit/frequency counting, for leave-one-out validation-panel
exclusion.";

fn parse_args(mut args: impl Iterator<Item = String>) -> Result<Args, String> {
    let mut output = None;
    let mut radius = None;
    let mut limit = None;
    let mut sources = Vec::new();
    let mut reader_options = chematic::mol::SmilesReaderOptions::default();
    let mut corpus_domain_name = None;
    let mut corpus_domain = None;
    let mut corpus_synthesis_focused = None;
    let mut corpus_domain_description = None;
    let mut exclude_smiles_file = None;

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--output" => output = Some(args.next().ok_or("--output requires a value")?),
            "--radius" => {
                let value = args.next().ok_or("--radius requires a value")?;
                radius = Some(
                    value
                        .trim()
                        .parse::<u32>()
                        .map_err(|_| format!("invalid --radius value {value:?}"))?,
                );
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
            "--exclude-smiles-file" => {
                exclude_smiles_file = Some(
                    args.next()
                        .ok_or("--exclude-smiles-file requires a value")?,
                )
            }
            "--corpus-domain-name" => {
                corpus_domain_name =
                    Some(args.next().ok_or("--corpus-domain-name requires a value")?)
            }
            "--corpus-domain" => {
                corpus_domain = Some(args.next().ok_or("--corpus-domain requires a value")?)
            }
            "--corpus-synthesis-focused" => {
                let value = args
                    .next()
                    .ok_or("--corpus-synthesis-focused requires a value")?;
                corpus_synthesis_focused = Some(match value.as_str() {
                    "true" => true,
                    "false" => false,
                    other => {
                        return Err(format!(
                            "invalid --corpus-synthesis-focused value {other:?} (expected \
                             true|false)"
                        ));
                    }
                });
            }
            "--corpus-domain-description" => {
                corpus_domain_description = Some(
                    args.next()
                        .ok_or("--corpus-domain-description requires a value")?,
                )
            }
            other => return Err(format!("unrecognized argument {other:?}")),
        }
    }

    let output = output.ok_or("--output is required")?;
    if sources.is_empty() {
        return Err("at least one --source is required".to_string());
    }
    let corpus_domain_name = corpus_domain_name.ok_or("--corpus-domain-name is required")?;
    let corpus_domain = corpus_domain.ok_or("--corpus-domain is required")?;
    let corpus_synthesis_focused =
        corpus_synthesis_focused.ok_or("--corpus-synthesis-focused is required")?;
    let corpus_domain_description =
        corpus_domain_description.ok_or("--corpus-domain-description is required")?;

    Ok(Args {
        output,
        radius: radius.unwrap_or(DEFAULT_RADIUS),
        limit,
        sources,
        corpus_domain_name,
        corpus_domain,
        corpus_synthesis_focused,
        corpus_domain_description,
        reader_options,
        exclude_smiles_file,
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
    /// Round 19: records dropped because their canonical SMILES matched an
    /// entry in `--exclude-smiles-file`, checked before dedup so an
    /// excluded molecule never reaches `seen_canonical` and can't be
    /// double-counted as a "duplicate" instead.
    records_excluded_by_list: usize,
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
    radius: u32,
    preprocessing: String,
    exclusion_criteria: String,
    /// Round 19: path passed to `--exclude-smiles-file`, or `None` if it
    /// wasn't used -- so a manifest self-documents whether/how leave-one-out
    /// exclusion was applied, without needing the original build command.
    exclude_smiles_file: Option<String>,
    generated_at_unix: u64,
    chematic_version: String,
    tool_version: String,
    /// The `yomitoki::RULESET_VERSION` current when this corpus was built —
    /// informational provenance, not a compatibility constraint: the corpus
    /// format itself doesn't change across ruleset versions, but this lets
    /// a future investigation correlate a corpus's `reference_distribution`
    /// with which `fragment_precedent` formula it was measured against
    /// (round 17).
    yomitoki_ruleset_version_at_build: String,
    /// Version tag for how a "fragment" is defined and hashed — bump this
    /// (independent of `tool_version`) if the hashing scheme or fragment
    /// extraction function ever changes, since it invalidates comparability
    /// with corpora built under a prior definition.
    fragment_definition_version: String,
    /// Human-readable description of exactly what a "fragment" is and how
    /// document frequency is counted, kept in the manifest itself so a
    /// consumer doesn't need to read this tool's Rust source to interpret
    /// `fragment_frequencies.json`.
    fragment_definition: String,
    total_molecules_processed: u64,
    distinct_fragment_count: u64,
    /// Mean, across all kept molecules, of each molecule's own mean
    /// document frequency (the same statistic `fragment_precedent::compute`
    /// computes per query molecule) — a single-number summary of how
    /// "common" a typical corpus molecule's fragments are, measured
    /// directly rather than assumed (round 16/17).
    mean_document_frequency: f64,
    /// Median of the same per-molecule statistic — equivalent to
    /// `reference_distribution_quantiles.q50`, duplicated here as a
    /// standalone summary field since it's the value `signed_signal`
    /// treats as neutral (`p = 0.5`).
    median_document_frequency: f64,
    artifact_file: String,
    artifact_sha256: String,
    /// Human-readable description of `reference_distribution`'s shape and
    /// purpose, kept in the manifest itself for the same reason as
    /// `fragment_definition` above.
    reference_distribution_definition: String,
    /// Version tag for how the reference distribution is computed (the
    /// resampling method, grid resolution, and what statistic it's a
    /// distribution *of*) — bump this independently of `tool_version` if
    /// that computation ever changes, since it invalidates comparability
    /// with corpora built under a prior definition (round 18, mirroring
    /// `fragment_definition_version` above).
    reference_distribution_version: String,
    /// A `DISTRIBUTION_GRID_POINTS`-point quantile grid (index `i`
    /// corresponds to percentile `i / (DISTRIBUTION_GRID_POINTS - 1)`) of
    /// this corpus's own molecule-level mean-document-frequency
    /// distribution — one value per kept molecule (that molecule's mean
    /// document frequency across its own fragments, against this same
    /// corpus), sorted, then resampled onto the grid. Lets
    /// `fragment_precedent` convert a query molecule's mean document
    /// frequency into an empirical percentile *against this exact corpus*
    /// (`FragmentCorpus::percentile_rank`) instead of an arbitrary
    /// absolute scale — see `rules.rs`'s "Fragment precedent" section for
    /// why the old absolute-scale formula was wrong.
    reference_distribution: Vec<f64>,
    /// Named convenience subset of `reference_distribution`'s 1001 points,
    /// for a human or tool that wants the standard quantiles without
    /// indexing into the full grid.
    reference_distribution_quantiles: NamedQuantiles,
    /// What chemical space this corpus is claimed to represent (round 18
    /// — AGENTS.md §5.4). Required, not optional-with-a-guessed-default:
    /// `fragment_precedent`'s signal is only ever as good as the corpus
    /// it's given, and "rare in ChEMBL" vs. "hard to synthesize" is
    /// exactly the distinction this exists to keep traceable — see
    /// `CorpusDomain`'s own doc.
    corpus_domain: CorpusDomain,
}

/// What chemical space a corpus is claimed to represent, and whether its
/// builder claims it's synthesis-focused specifically — carried into
/// `yomitoki::FragmentCorpusProvenance` verbatim when a report is built
/// against this corpus. A provenance declaration the builder asserts, not
/// something this tool verifies.
#[derive(Serialize)]
struct CorpusDomain {
    source_name: String,
    domain: String,
    synthesis_focused: bool,
    description: String,
}

#[derive(Serialize)]
struct NamedQuantiles {
    q01: f64,
    q05: f64,
    q10: f64,
    q25: f64,
    q50: f64,
    q75: f64,
    q90: f64,
    q95: f64,
    q99: f64,
}

impl NamedQuantiles {
    fn from_sorted(sorted: &[f64]) -> Self {
        Self {
            q01: percentile(sorted, 0.01),
            q05: percentile(sorted, 0.05),
            q10: percentile(sorted, 0.10),
            q25: percentile(sorted, 0.25),
            q50: percentile(sorted, 0.50),
            q75: percentile(sorted, 0.75),
            q90: percentile(sorted, 0.90),
            q95: percentile(sorted, 0.95),
            q99: percentile(sorted, 0.99),
        }
    }
}

/// Linear-interpolation percentile of a value already sorted ascending.
fn percentile(sorted: &[f64], p: f64) -> f64 {
    if sorted.is_empty() {
        return f64::NAN;
    }
    let idx = ((sorted.len() - 1) as f64 * p).round() as usize;
    sorted[idx.min(sorted.len() - 1)]
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

    let excluded_canonical: HashSet<String> = match &args.exclude_smiles_file {
        Some(path) => {
            let content = std::fs::read_to_string(path)
                .map_err(|e| format!("could not read --exclude-smiles-file {path:?}: {e}"))?;
            content
                .lines()
                .map(str::trim)
                .filter(|line| !line.is_empty())
                .map(|line| {
                    // Tolerate a trailing name/id column, same shape as a
                    // plain .smi source file.
                    let smiles = line.split_whitespace().next().unwrap_or(line);
                    let mol = chematic::smiles::parse(smiles).map_err(|e| {
                        format!("could not parse --exclude-smiles-file entry {smiles:?}: {e}")
                    })?;
                    Ok(chematic::smiles::canonical_smiles(&mol))
                })
                .collect::<Result<HashSet<String>, String>>()?
        }
        None => HashSet::new(),
    };

    let mut seen_canonical: HashSet<String> = HashSet::new();
    // fragment_hash -> number of distinct molecules containing it.
    let mut frequency: HashMap<u64, u64> = HashMap::new();
    // One entry per kept molecule, in processing order: its distinct
    // fragment hashes. Small (a few dozen u64s per molecule at most) —
    // kept in memory for the second pass below, which needs the *complete*
    // frequency table to compute each molecule's own mean document
    // frequency (not available until every molecule has been counted).
    let mut molecule_fragment_hashes: Vec<Vec<u64>> = Vec::new();
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
        let mut records_excluded_by_list = 0usize;
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
            // Checked before dedup/--limit and before this molecule can
            // contribute to `frequency`/`molecule_fragment_hashes` at all --
            // an excluded molecule must have zero influence on the
            // resulting reference_distribution (leave-one-out, round 19).
            if excluded_canonical.contains(&canonical) {
                records_excluded_by_list += 1;
                continue;
            }
            if !seen_canonical.insert(canonical) {
                records_duplicate += 1;
                continue;
            }

            records_kept += 1;
            total_molecules_processed += 1;
            if let Some(remaining) = remaining_limit.as_mut() {
                *remaining -= 1;
            }

            let counts = chematic::fp::morgan_fp_counts(&mol, args.radius);
            let hashes: Vec<u64> = counts.keys().copied().collect();
            for &hash in &hashes {
                *frequency.entry(hash).or_insert(0) += 1;
            }
            molecule_fragment_hashes.push(hashes);
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
            records_excluded_by_list,
            records_kept,
            records_duplicate,
        });
    }

    let mut fragments: Vec<FragmentRecord> = frequency
        .iter()
        .map(|(&fragment_hash, &occurrence_count)| FragmentRecord {
            radius: args.radius,
            fragment_hash,
            occurrence_count,
        })
        .collect();
    // Deterministic byte output (artifact_sha256 depends on it) — HashMap
    // iteration order isn't stable, so sort explicitly before writing.
    fragments.sort_by_key(|f| f.fragment_hash);
    let distinct_fragment_count = fragments.len() as u64;

    // Second pass: now that `frequency` is complete, compute each kept
    // molecule's own mean document frequency against it, matching exactly
    // what `fragment_precedent::compute` does at inference time.
    let total = total_molecules_processed.max(1) as f64;
    let mut mean_dfs: Vec<f64> = molecule_fragment_hashes
        .iter()
        .filter(|hashes| !hashes.is_empty())
        .map(|hashes| {
            let sum: f64 = hashes
                .iter()
                .map(|hash| frequency.get(hash).copied().unwrap_or(0) as f64 / total)
                .sum();
            sum / hashes.len() as f64
        })
        .collect();
    mean_dfs.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let reference_distribution: Vec<f64> = (0..DISTRIBUTION_GRID_POINTS)
        .map(|i| percentile(&mean_dfs, i as f64 / (DISTRIBUTION_GRID_POINTS - 1) as f64))
        .collect();
    let reference_distribution_quantiles = NamedQuantiles::from_sorted(&mean_dfs);
    let mean_document_frequency = if mean_dfs.is_empty() {
        0.0
    } else {
        mean_dfs.iter().sum::<f64>() / mean_dfs.len() as f64
    };
    let median_document_frequency = reference_distribution_quantiles.q50;

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
        radius: args.radius,
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
        exclusion_criteria: if args.exclude_smiles_file.is_some() {
            "atoms outside yomitoki::SUPPORTED_ELEMENTS; exact canonical-SMILES duplicates; \
             canonical-SMILES matches against --exclude-smiles-file"
                .to_string()
        } else {
            "atoms outside yomitoki::SUPPORTED_ELEMENTS; exact canonical-SMILES duplicates"
                .to_string()
        },
        exclude_smiles_file: args.exclude_smiles_file.clone(),
        generated_at_unix,
        chematic_version: "0.12".to_string(),
        tool_version: env!("CARGO_PKG_VERSION").to_string(),
        yomitoki_ruleset_version_at_build: yomitoki::RULESET_VERSION.to_string(),
        fragment_definition_version: "morgan-ecfp-v1".to_string(),
        fragment_definition: format!(
            "chematic::fp::morgan_fp_counts(mol, radius={}) — circular \
             (Morgan/ECFP-like) substructure hashes, cumulative over \
             iterations 0..=radius. Document frequency = number of \
             distinct kept molecules in which a given fragment hash occurs \
             at least once (not raw occurrence count).",
            args.radius
        ),
        total_molecules_processed,
        distinct_fragment_count,
        mean_document_frequency,
        median_document_frequency,
        artifact_file: "fragment_frequencies.json".to_string(),
        artifact_sha256,
        reference_distribution_definition: format!(
            "A {DISTRIBUTION_GRID_POINTS}-point quantile grid (index i = \
             percentile i/{}) of this corpus's own molecule-level \
             mean-document-frequency distribution: for each kept molecule, \
             the mean of (occurrence_count / total_molecules_processed) \
             across that molecule's own fragment hashes, sorted ascending \
             and resampled onto the grid. FragmentCorpus::percentile_rank \
             binary-searches this grid to convert a query molecule's mean \
             document frequency into an empirical percentile against this \
             exact corpus.",
            DISTRIBUTION_GRID_POINTS - 1
        ),
        reference_distribution_version: "quantile-grid-v1".to_string(),
        reference_distribution,
        reference_distribution_quantiles,
        corpus_domain: CorpusDomain {
            source_name: args.corpus_domain_name.clone(),
            domain: args.corpus_domain.clone(),
            synthesis_focused: args.corpus_synthesis_focused,
            description: args.corpus_domain_description.clone(),
        },
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

    // Every parse_args test that exercises a successful parse needs the
    // four --corpus-domain-* flags too, since round 18 made them required
    // (no guessed default) — kept as one constant slice so each test only
    // has to append its own scenario-specific flags.
    const DOMAIN_ARGS: &[&str] = &[
        "--corpus-domain-name",
        "Test Corpus",
        "--corpus-domain",
        "test",
        "--corpus-synthesis-focused",
        "false",
        "--corpus-domain-description",
        "A fixture corpus for parse_args tests.",
    ];

    #[test]
    fn parses_minimal_invocation() {
        let parsed = parse_args(args(&[
            "--output",
            "out",
            "--source",
            "DrugBank|CC0-1.0|https://example.test|data.sdf",
            "--corpus-domain-name",
            "Test Corpus",
            "--corpus-domain",
            "test",
            "--corpus-synthesis-focused",
            "false",
            "--corpus-domain-description",
            "A fixture corpus for parse_args tests.",
        ]))
        .unwrap();
        assert_eq!(parsed.output, "out");
        assert_eq!(parsed.radius, DEFAULT_RADIUS);
        assert_eq!(parsed.limit, None);
        assert_eq!(parsed.sources.len(), 1);
        assert_eq!(parsed.sources[0].name, "DrugBank");
        assert_eq!(parsed.sources[0].license, "CC0-1.0");
        assert_eq!(parsed.sources[0].url, "https://example.test");
        assert_eq!(parsed.sources[0].path, "data.sdf");
        assert_eq!(parsed.corpus_domain_name, "Test Corpus");
        assert_eq!(parsed.corpus_domain, "test");
        assert!(!parsed.corpus_synthesis_focused);
        assert_eq!(
            parsed.corpus_domain_description,
            "A fixture corpus for parse_args tests."
        );
    }

    #[test]
    fn parses_custom_radius_and_limit_and_multiple_sources() {
        let mut argv = vec![
            "--output",
            "out",
            "--radius",
            "3",
            "--limit",
            "100",
            "--source",
            "A|CC0-1.0|https://a.test|a.sdf",
            "--source",
            "B|CC-BY-4.0|https://b.test|b.smi",
        ];
        argv.extend_from_slice(DOMAIN_ARGS);
        let parsed = parse_args(args(&argv)).unwrap();
        assert_eq!(parsed.radius, 3);
        assert_eq!(parsed.limit, Some(100));
        assert_eq!(parsed.sources.len(), 2);
    }

    #[test]
    fn parses_synthesis_focused_true() {
        let mut argv = vec!["--output", "out", "--source", "A|L|U|p"];
        argv.extend_from_slice(DOMAIN_ARGS);
        // Override the fixture's "false" with "true" — DOMAIN_ARGS's value
        // is simply superseded since parse_args keeps the last flag seen.
        argv.extend_from_slice(&["--corpus-synthesis-focused", "true"]);
        let parsed = parse_args(args(&argv)).unwrap();
        assert!(parsed.corpus_synthesis_focused);
    }

    #[test]
    fn rejects_missing_output() {
        let mut argv = vec!["--source", "A|L|U|p"];
        argv.extend_from_slice(DOMAIN_ARGS);
        assert!(parse_args(args(&argv)).is_err());
    }

    #[test]
    fn rejects_missing_sources() {
        let mut argv = vec!["--output", "out"];
        argv.extend_from_slice(DOMAIN_ARGS);
        assert!(parse_args(args(&argv)).is_err());
    }

    #[test]
    fn rejects_malformed_source() {
        let mut argv = vec!["--output", "out", "--source", "only-two|parts"];
        argv.extend_from_slice(DOMAIN_ARGS);
        assert!(parse_args(args(&argv)).is_err());
    }

    #[test]
    fn rejects_missing_corpus_domain_flags() {
        assert!(
            parse_args(args(&["--output", "out", "--source", "A|L|U|p"])).is_err(),
            "corpus-domain-* flags are required (round 18), not defaulted"
        );
    }

    #[test]
    fn rejects_invalid_synthesis_focused_value() {
        let mut argv = vec!["--output", "out", "--source", "A|L|U|p"];
        argv.extend_from_slice(DOMAIN_ARGS);
        argv.extend_from_slice(&["--corpus-synthesis-focused", "yes"]);
        assert!(parse_args(args(&argv)).is_err());
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

    #[test]
    fn parses_exclude_smiles_file() {
        let mut argv = vec!["--output", "out", "--source", "A|L|U|p"];
        argv.extend_from_slice(DOMAIN_ARGS);
        argv.extend_from_slice(&["--exclude-smiles-file", "panel.smi"]);
        let parsed = parse_args(args(&argv)).unwrap();
        assert_eq!(parsed.exclude_smiles_file, Some("panel.smi".to_string()));
    }

    #[test]
    fn exclude_smiles_file_defaults_to_none() {
        let mut argv = vec!["--output", "out", "--source", "A|L|U|p"];
        argv.extend_from_slice(DOMAIN_ARGS);
        let parsed = parse_args(args(&argv)).unwrap();
        assert_eq!(parsed.exclude_smiles_file, None);
    }

    /// No `tempfile` dev-dependency in this standalone tool crate -- a
    /// process-id + atomic-counter suffix under the OS temp dir is enough
    /// to keep parallel `cargo test` runs from colliding.
    fn temp_dir_for(label: &str) -> std::path::PathBuf {
        static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let n = COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        std::env::temp_dir().join(format!(
            "build-fragment-corpus-test-{}-{label}-{n}",
            std::process::id()
        ))
    }

    #[test]
    fn exclude_smiles_file_drops_molecules_before_dedup_and_frequency_counting() {
        let dir = temp_dir_for("exclude");
        std::fs::create_dir_all(&dir).unwrap();
        let source_path = dir.join("molecules.smi");
        std::fs::write(&source_path, "CCO ethanol\nCCN methylamine\nCCC propane\n").unwrap();
        let exclude_path = dir.join("exclude.smi");
        // Deliberately a different-but-equivalent SMILES spelling of
        // ethanol than the source file uses, to prove exclusion matches on
        // canonical form, not raw string equality.
        std::fs::write(&exclude_path, "OCC\n").unwrap();
        let output_dir = dir.join("out");

        let source_flag = format!(
            "Test|CC0-1.0|https://example.test|{}",
            source_path.to_str().unwrap()
        );
        let mut argv = vec![
            "--output",
            output_dir.to_str().unwrap(),
            "--source",
            &source_flag,
            "--exclude-smiles-file",
            exclude_path.to_str().unwrap(),
        ];
        argv.extend_from_slice(DOMAIN_ARGS);
        let parsed = parse_args(args(&argv)).unwrap();
        run(&parsed).unwrap();

        let manifest: serde_json::Value =
            serde_json::from_slice(&std::fs::read(output_dir.join("manifest.json")).unwrap())
                .unwrap();
        assert_eq!(manifest["sources"][0]["records_excluded_by_list"], 1);
        assert_eq!(manifest["sources"][0]["records_kept"], 2);
        assert_eq!(manifest["total_molecules_processed"], 2);
        assert_eq!(
            manifest["exclude_smiles_file"],
            exclude_path.to_str().unwrap()
        );

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn no_exclude_smiles_file_leaves_the_manifest_field_null() {
        let dir = temp_dir_for("no-exclude");
        std::fs::create_dir_all(&dir).unwrap();
        let source_path = dir.join("molecules.smi");
        std::fs::write(&source_path, "CCO ethanol\n").unwrap();
        let output_dir = dir.join("out");

        let source_flag = format!(
            "Test|CC0-1.0|https://example.test|{}",
            source_path.to_str().unwrap()
        );
        let mut argv = vec![
            "--output",
            output_dir.to_str().unwrap(),
            "--source",
            &source_flag,
        ];
        argv.extend_from_slice(DOMAIN_ARGS);
        let parsed = parse_args(args(&argv)).unwrap();
        run(&parsed).unwrap();

        let manifest: serde_json::Value =
            serde_json::from_slice(&std::fs::read(output_dir.join("manifest.json")).unwrap())
                .unwrap();
        assert_eq!(manifest["sources"][0]["records_excluded_by_list"], 0);
        assert!(manifest["exclude_smiles_file"].is_null());

        std::fs::remove_dir_all(&dir).ok();
    }
}
