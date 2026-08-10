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
    version: String,
}

impl FragmentCorpus {
    /// Loads `<dir>/fragment_frequencies.json` and `<dir>/manifest.json`,
    /// the exact file names `tools/build-fragment-corpus --output <dir>`
    /// writes.
    ///
    /// The frequency table must reference exactly one radius — build with
    /// `--radii <N>` (a single value), not the tool's discouraged `0,1,2`
    /// default (`chematic::fp::morgan_fp_counts` is cumulative, so multiple
    /// radii store the same underlying fragments redundantly; see the
    /// tool's README).
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
                         single --radii value",
                        record.radius
                    )));
                }
                _ => {}
            }
            frequency.insert(record.fragment_hash, record.occurrence_count);
        }
        let radius = radius
            .ok_or_else(|| YomitokiError::ModelLoadError("corpus has no fragments".to_string()))?;

        Ok(FragmentCorpus {
            radius,
            total_molecules_processed: table.total_molecules_processed,
            frequency,
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
}

fn read_json<T: serde::de::DeserializeOwned>(path: &Path) -> Result<T, YomitokiError> {
    let bytes = std::fs::read(path)
        .map_err(|e| YomitokiError::ModelLoadError(format!("could not read {path:?}: {e}")))?;
    serde_json::from_slice(&bytes)
        .map_err(|e| YomitokiError::ModelLoadError(format!("could not parse {path:?}: {e}")))
}
