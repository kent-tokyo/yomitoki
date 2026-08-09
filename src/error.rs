//! Error type. AGENTS.md §17: a hard-to-synthesize or out-of-domain molecule
//! is never an error — only a truly unparseable/misconfigured input is.

use std::fmt;

/// Only the variants reachable by code that exists today. The rest of
/// AGENTS.md §17's sketch (`UnsupportedMolecule`, `ModelLoadError`,
/// `InternalInvariantViolation`) is added alongside the code that can
/// actually raise them.
#[non_exhaustive]
#[derive(Debug)]
pub enum RenseiError {
    ParseError(chematic::smiles::SmilesError),
    InvalidConfiguration(String),
}

impl fmt::Display for RenseiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RenseiError::ParseError(e) => write!(f, "failed to parse molecule: {e}"),
            RenseiError::InvalidConfiguration(msg) => write!(f, "invalid configuration: {msg}"),
        }
    }
}

impl std::error::Error for RenseiError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            RenseiError::ParseError(e) => Some(e),
            RenseiError::InvalidConfiguration(_) => None,
        }
    }
}

impl From<chematic::smiles::SmilesError> for RenseiError {
    fn from(e: chematic::smiles::SmilesError) -> Self {
        RenseiError::ParseError(e)
    }
}
