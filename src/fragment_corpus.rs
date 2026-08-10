//! Loads a fragment-frequency corpus built by `tools/build-fragment-corpus`
//! (AGENTS.md §5.4) for the `fragment_rarity` component.
//!
//! No corpus ships with `yomitoki` itself — AGENTS.md §5.4 forbids
//! embedding one directly in the library as a huge binary, and no decision
//! has been made yet about where a built corpus would ship from (see
//! `tasks/upstream_and_corpus_research.md`, gitignored). `fragment_rarity`
//! stays `None` unless a caller explicitly builds a corpus (via the tool
//! above) and loads it here.
//!
//! Loading is a separate, explicitly fallible step from `analyze` itself
//! (AGENTS.md §17: parsing is the only fallible step inside `analyze`) —
//! load a corpus once, attach it to `AnalysisConfig::fragment_model`, and
//! every subsequent `analyze`/`analyze_smiles`/`analyze_batch` call reuses
//! the already-loaded data with no I/O and no fallibility of its own.

use std::collections::HashMap;
use std::path::Path;

use serde::Deserialize;

use crate::error::YomitokiError;

#[derive(Deserialize)]
struct FragmentRecord {
    radius: u32,
    fragment_hash: u64,
    occurrence_count: u64,
}

#[derive(Deserialize)]
struct FrequencyTableFile {
    total_molecules_processed: u64,
    fragments: Vec<FragmentRecord>,
}

#[derive(Deserialize)]
struct ManifestFile {
    artifact_sha256: String,
    reference_distribution: Vec<f64>,
}

/// A loaded fragment-frequency corpus, as produced by
/// `tools/build-fragment-corpus`. Attach one via
/// [`crate::config::FragmentModelConfig`] to enable the `fragment_rarity`
/// component.
#[derive(Debug, Clone, PartialEq)]
pub struct FragmentCorpus {
    pub(crate) radius: u32,
    pub(crate) total_molecules_processed: u64,
    pub(crate) frequency: HashMap<u64, u64>,
    /// Sorted quantile grid of this corpus's own molecule-level mean
    /// -document-frequency distribution — index `i` is the value at
    /// percentile `i / (len - 1)`. See [`FragmentCorpus::percentile_rank`].
    reference_distribution: Vec<f64>,
    version: String,
}

impl FragmentCorpus {
    /// Loads `<dir>/fragment_frequencies.json` and `<dir>/manifest.json`,
    /// the exact file names `tools/build-fragment-corpus --output <dir>`
    /// writes.
    ///
    /// The frequency table must reference exactly one radius — build with
    /// `--radius <N>` (a single value; the tool's flag was a list before
    /// round 17, since `chematic::fp::morgan_fp_counts` is cumulative and
    /// multiple radii would store the same fragments redundantly). The
    /// manifest must carry a non-empty `reference_distribution` — corpora
    /// built before round 17 don't have one and need rebuilding; there is
    /// no absolute-scale fallback (see `rules::FRAGMENT_RARITY_WEIGHT`'s
    /// doc comment for why an absolute scale doesn't work).
    pub fn load_dir(dir: impl AsRef<Path>) -> Result<FragmentCorpus, YomitokiError> {
        let dir = dir.as_ref();
        let table: FrequencyTableFile = read_json(&dir.join("fragment_frequencies.json"))?;
        let manifest: ManifestFile = read_json(&dir.join("manifest.json"))?;

        let mut radius = None;
        let mut frequency = HashMap::with_capacity(table.fragments.len());
        for record in table.fragments {
            match radius {
                None => radius = Some(record.radius),
                Some(r) if r != record.radius => {
                    return Err(YomitokiError::ModelLoadError(format!(
                        "corpus references multiple radii ({r} and {}) — rebuild with a \
                         single --radius value",
                        record.radius
                    )));
                }
                _ => {}
            }
            frequency.insert(record.fragment_hash, record.occurrence_count);
        }
        let radius = radius
            .ok_or_else(|| YomitokiError::ModelLoadError("corpus has no fragments".to_string()))?;
        if manifest.reference_distribution.len() < 2 {
            return Err(YomitokiError::ModelLoadError(
                "corpus manifest has no reference_distribution — rebuild with \
                 tools/build-fragment-corpus (round 17 or later)"
                    .to_string(),
            ));
        }

        Ok(FragmentCorpus {
            radius,
            total_molecules_processed: table.total_molecules_processed,
            frequency,
            reference_distribution: manifest.reference_distribution,
            version: manifest.artifact_sha256,
        })
    }

    /// The corpus identifier reported in `Provenance.model_version` —
    /// currently the built artifact's `artifact_sha256` (see
    /// `tools/build-fragment-corpus`'s manifest), so two reports can be
    /// compared knowing whether they used the same corpus.
    pub fn version(&self) -> &str {
        &self.version
    }

    /// Where `mean_document_frequency` falls within this corpus's own
    /// molecule-level mean-document-frequency distribution, as an
    /// empirical percentile in `0.0..=1.0` (linear interpolation between
    /// the two nearest grid points). `0.0` = at or below the rarest
    /// molecule this corpus has seen; `1.0` = at or above the most common.
    /// This is what makes `fragment_rarity`'s signal corpus-*relative*
    /// rather than an absolute scale no real corpus approaches the
    /// extremes of (see `rules::FRAGMENT_RARITY_WEIGHT`'s doc comment).
    pub(crate) fn percentile_rank(&self, mean_document_frequency: f64) -> f64 {
        let grid = &self.reference_distribution;
        match grid.binary_search_by(|v| v.partial_cmp(&mean_document_frequency).unwrap()) {
            Ok(i) => i as f64 / (grid.len() - 1) as f64,
            Err(0) => 0.0,
            Err(i) if i >= grid.len() => 1.0,
            Err(i) => {
                // Linear interpolation between grid[i-1] and grid[i].
                let (lo, hi) = (grid[i - 1], grid[i]);
                let frac = if hi > lo {
                    (mean_document_frequency - lo) / (hi - lo)
                } else {
                    0.0
                };
                ((i - 1) as f64 + frac) / (grid.len() - 1) as f64
            }
        }
        .clamp(0.0, 1.0)
    }
}

fn read_json<T: serde::de::DeserializeOwned>(path: &Path) -> Result<T, YomitokiError> {
    let bytes = std::fs::read(path)
        .map_err(|e| YomitokiError::ModelLoadError(format!("could not read {path:?}: {e}")))?;
    serde_json::from_slice(&bytes)
        .map_err(|e| YomitokiError::ModelLoadError(format!("could not parse {path:?}: {e}")))
}
