//! Error type. AGENTS.md §17: a hard-to-synthesize or out-of-domain molecule
//! is never an error — only a truly unparseable/misconfigured input is.

use std::fmt;

/// Only the variants reachable by code that exists today. The rest of
/// AGENTS.md §17's sketch (`UnsupportedMolecule`, `InternalInvariantViolation`)
/// is added alongside the code that can actually raise them.
#[non_exhaustive]
#[derive(Debug)]
pub enum YomitokiError {
    /// The input SMILES could not be parsed by chematic.
    ParseError(chematic::smiles::SmilesError),
    /// `AnalysisConfig` itself was invalid (not currently reachable — no
    /// v0.1 config field has invalid states to detect yet).
    InvalidConfiguration(String),
    /// A fragment corpus (`FragmentCorpus::load_dir`) could not be read or
    /// parsed. Deliberately a separate, explicit step from `analyze` itself
    /// (AGENTS.md §17: parsing is the only fallible step inside `analyze`)
    /// — a corpus is loaded once, up front, not lazily on every call.
    ModelLoadError(String),
}

impl fmt::Display for YomitokiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            YomitokiError::ParseError(e) => write!(f, "failed to parse molecule: {e}"),
            YomitokiError::InvalidConfiguration(msg) => write!(f, "invalid configuration: {msg}"),
            YomitokiError::ModelLoadError(msg) => write!(f, "could not load model: {msg}"),
        }
    }
}

impl std::error::Error for YomitokiError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            YomitokiError::ParseError(e) => Some(e),
            YomitokiError::InvalidConfiguration(_) => None,
            YomitokiError::ModelLoadError(_) => None,
        }
    }
}

impl From<chematic::smiles::SmilesError> for YomitokiError {
    fn from(e: chematic::smiles::SmilesError) -> Self {
        YomitokiError::ParseError(e)
    }
}
