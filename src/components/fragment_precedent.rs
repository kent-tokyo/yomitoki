//! Fragment precedent component (AGENTS.md §5.4) — corpus-relative signed
//! precedent (round 17 redesign; round 18 rename from `fragment_rarity`,
//! since it now argues difficulty both up *and* down, not just up — see
//! `rules.rs`'s "Fragment precedent" section for the full formula
//! derivation and round 16's finding that the original absolute-scale
//! formula was confirmed broken end-to-end).
//!
//! Only runs when `AnalysisConfig.fragment_model` has a
//! [`crate::FragmentCorpus`] configured — no corpus ships with yomitoki
//! itself (AGENTS.md §5.4: never embed one directly in the library as a
//! huge binary), so this component is opt-in, not always-on like the other
//! four (see `docs/architecture.md`).
//!
//! Fragments are `chematic::fp::morgan_fp_counts(mol, corpus.radius)` —
//! circular/ECFP-like atom environments, already the corpus-build
//! pipeline's own choice (`tools/build-fragment-corpus`). This molecule's
//! *mean* document frequency across those fragments (mean over minimum —
//! round 14's finding, still valid under the new formula) is converted to
//! an empirical percentile against the corpus's own molecule-level
//! distribution (`FragmentCorpus::percentile_rank`), then split into a
//! weak-precedent penalty (below the corpus median) or precedent support
//! (above it).
//!
//! `analyze::analyze` builds the final `FragmentPrecedentEvidence` from
//! this module's raw numbers. Round 21 (option C): no cap is applied
//! anymore, and neither `precedent_penalty` nor `precedent_support`
//! contributes to `overall.difficulty` — round 20's cross-corpus
//! robustness test found the signal too corpus-sensitive to trust as a
//! scoring input (two honestly-labeled synthesis-focused corpora
//! disagreed with each other on its direction 34.6% of the time over 500
//! probe molecules; see `rules.rs`'s "Fragment precedent" section for the
//! full history). This module's own formula is unchanged since round 17 —
//! the finding was about the *contract* (should this feed a score at
//! all), not this computation.

use chematic::core::Molecule;

use crate::fragment_corpus::FragmentCorpus;
use crate::report::{Finding, FindingCode, FindingEvidence, ProbabilityLikeScore, Severity};
use crate::rules::FRAGMENT_PRECEDENT_FINDING_THRESHOLD;

pub(crate) struct FragmentPrecedentOutcome {
    /// `max(signed_signal, 0.0)`. Reported as explanatory evidence only
    /// since round 21 — never applied to `overall.difficulty` (rounds
    /// 17-19 applied it in full, uncapped; see `rules.rs`'s "Fragment
    /// precedent" section for why that was found unsafe).
    pub(crate) precedent_penalty: f64,
    /// `max(-signed_signal, 0.0)`. Reported as explanatory evidence only
    /// since round 21 — never applied to `overall.difficulty` (rounds
    /// 17-19 capped and applied it; the cap existed only to bound this
    /// signal's effect on difficulty, which it no longer has, so there is
    /// no cap to apply anymore).
    pub(crate) precedent_support: f64,
    /// At most one — `FragmentPrecedentWeak` (penalty) or
    /// `FragmentPrecedentStrong` (support), only when `|signed_signal|`
    /// clears `FRAGMENT_PRECEDENT_FINDING_THRESHOLD` (a display threshold;
    /// `precedent_penalty`/`precedent_support` above still apply below it).
    pub(crate) finding: Option<Finding>,
}

fn neutral_outcome() -> FragmentPrecedentOutcome {
    FragmentPrecedentOutcome {
        precedent_penalty: 0.0,
        precedent_support: 0.0,
        finding: None,
    }
}

pub(crate) fn compute(mol: &Molecule, corpus: &FragmentCorpus) -> FragmentPrecedentOutcome {
    let counts = chematic::fp::morgan_fp_counts(mol, corpus.radius);
    if counts.is_empty() {
        // No atom environments at all (e.g. a single-atom molecule) — no
        // signal to score, matching every other component's "no data, no
        // finding" behavior rather than a fabricated score.
        return neutral_outcome();
    }

    let total = corpus.total_molecules_processed.max(1) as f64;
    let mean_document_frequency = counts
        .keys()
        .map(|hash| corpus.frequency.get(hash).copied().unwrap_or(0) as f64 / total)
        .sum::<f64>()
        / counts.len() as f64;

    let percentile = corpus.percentile_rank(mean_document_frequency);
    let signed_signal = (1.0 - 2.0 * percentile).clamp(-1.0, 1.0);
    let precedent_penalty = signed_signal.max(0.0);
    let precedent_support = (-signed_signal).max(0.0);

    let finding = if signed_signal.abs() >= FRAGMENT_PRECEDENT_FINDING_THRESHOLD {
        let code = if signed_signal > 0.0 {
            FindingCode::FragmentPrecedentWeak
        } else {
            FindingCode::FragmentPrecedentStrong
        };
        let evidence = FindingEvidence {
            value: Some(percentile * 100.0),
            threshold: None,
        };
        let explanation = crate::explain::render(code, evidence, 0, None);
        Some(Finding {
            code,
            severity: Severity::Low,
            // Deterministic given a fixed corpus — no sampling
            // -uncertainty model exists yet for "how much does this
            // corpus's size affect confidence in a percentile estimate"
            // (a real gap, not modeled in v0.1; see docs/architecture.md).
            confidence: ProbabilityLikeScore::new(1.0),
            atoms: Vec::new(),
            evidence,
            explanation,
        })
    } else {
        None
    };

    FragmentPrecedentOutcome {
        precedent_penalty,
        precedent_support,
        finding,
    }
}
