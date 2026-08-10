//! Fragment rarity component (AGENTS.md §5.4).
//!
//! Only runs when `AnalysisConfig.fragment_model` has a
//! [`crate::FragmentCorpus`] configured — no corpus ships with yomitoki
//! itself (AGENTS.md §5.4: never embed one directly in the library as a
//! huge binary), so this component is opt-in, not always-on like the other
//! four (see `docs/architecture.md`).
//!
//! Fragments are `chematic::fp::morgan_fp_counts(mol, corpus.radius)` —
//! circular/ECFP-like atom environments, one of §5.4's acceptable v0.1
//! fragmentation approaches, and already the corpus-build pipeline's own
//! choice (`tools/build-fragment-corpus`). Rarity is scored from *mean*
//! document frequency across a molecule's fragments, not minimum — see
//! `rules::FRAGMENT_RARITY_WEIGHT`'s doc for why (round-14 corpus testing
//! found minimum alone doesn't separate known-common molecules from an
//! atypical control; mean does).

use chematic::core::Molecule;

use crate::fragment_corpus::FragmentCorpus;
use crate::report::{
    ComponentScore, Contribution, Finding, FindingCode, FindingEvidence, FindingRef,
    ProbabilityLikeScore, Severity, finite_or_zero,
};
use crate::rules::{
    FRAGMENT_RARITY_BURDEN_SCALE, FRAGMENT_RARITY_DF_THRESHOLD, FRAGMENT_RARITY_REPORT_COUNT,
    FRAGMENT_RARITY_WEIGHT,
};

pub(crate) struct FragmentRarityOutcome {
    pub(crate) score: ComponentScore,
    pub(crate) findings: Vec<Finding>,
    pub(crate) contributions: Vec<Contribution>,
}

fn empty_outcome() -> FragmentRarityOutcome {
    FragmentRarityOutcome {
        score: ComponentScore {
            raw: 0.0,
            normalized: ProbabilityLikeScore::new(0.0),
            confidence: ProbabilityLikeScore::new(1.0),
            contribution: ProbabilityLikeScore::new(0.0),
            findings: Vec::new(),
        },
        findings: Vec::new(),
        contributions: Vec::new(),
    }
}

pub(crate) fn compute(mol: &Molecule, corpus: &FragmentCorpus) -> FragmentRarityOutcome {
    let counts = chematic::fp::morgan_fp_counts(mol, corpus.radius);
    if counts.is_empty() {
        // No atom environments at all (e.g. a single-atom molecule) — no
        // signal to score, matching every other component's "no data, no
        // finding" behavior rather than a fabricated score.
        return empty_outcome();
    }

    let total = corpus.total_molecules_processed.max(1) as f64;
    let mut document_frequencies: Vec<(u64, f64)> = counts
        .keys()
        .map(|hash| {
            let occurrence = corpus.frequency.get(hash).copied().unwrap_or(0);
            (*hash, occurrence as f64 / total)
        })
        .collect();
    document_frequencies.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));

    let mean_df = document_frequencies.iter().map(|(_, df)| df).sum::<f64>()
        / document_frequencies.len() as f64;
    let rare_count = document_frequencies
        .iter()
        .filter(|(_, df)| *df < FRAGMENT_RARITY_DF_THRESHOLD)
        .count();

    // Continuous, unconditional on any threshold — same pattern as
    // size_topology's mw_weight/rotatable_weight, which contribute to
    // `raw` regardless of whether their own threshold-triggered finding
    // exists. `rare_count`/the finding below are supplementary evidence,
    // not what drives this value (see rules::FRAGMENT_RARITY_WEIGHT's doc).
    let raw = finite_or_zero(FRAGMENT_RARITY_WEIGHT * (1.0 - mean_df));

    let mut findings = Vec::new();
    let mut contributions = Vec::new();

    if rare_count > 0 {
        let rarest_label = document_frequencies
            .iter()
            .take(FRAGMENT_RARITY_REPORT_COUNT)
            .map(|(hash, df)| {
                let occurrence = (df * total).round() as u64;
                format!(
                    "{hash:016x} ({occurrence}/{} molecules, {:.4}%)",
                    total as u64,
                    df * 100.0
                )
            })
            .collect::<Vec<_>>()
            .join(", ");

        let evidence = FindingEvidence {
            value: Some(rare_count as f64),
            threshold: Some(FRAGMENT_RARITY_DF_THRESHOLD),
        };
        let explanation = crate::explain::render(
            FindingCode::FragmentRarityHigh,
            evidence,
            0,
            Some(&rarest_label),
        );
        findings.push(Finding {
            code: FindingCode::FragmentRarityHigh,
            severity: Severity::Low,
            confidence: ProbabilityLikeScore::new(1.0),
            atoms: Vec::new(),
            evidence,
            explanation: explanation.clone(),
        });
        // The raw weight this one finding is responsible for — same role
        // as size_topology's mw_weight/rotatable_weight (the pre-transform
        // number, not a re-normalized one; `dominant_penalties` ranks by
        // this).
        contributions.push(Contribution {
            code: FindingCode::FragmentRarityHigh,
            name: explanation,
            contribution: ProbabilityLikeScore::new(raw),
        });
    }

    // Non-linear burden (AGENTS.md §5.1), same saturating transform as
    // every other component.
    let normalized = ProbabilityLikeScore::new(1.0 - (-raw / FRAGMENT_RARITY_BURDEN_SCALE).exp());

    let score = ComponentScore {
        raw,
        normalized,
        // Deterministic given a fixed corpus — no sampling-uncertainty
        // model exists yet for "how much does this corpus's size affect
        // confidence in an unseen fragment being genuinely rare" (a real
        // gap, not modeled in v0.1; see docs/architecture.md).
        confidence: ProbabilityLikeScore::new(1.0),
        contribution: normalized,
        findings: (0..findings.len()).map(FindingRef).collect(),
    };

    FragmentRarityOutcome {
        score,
        findings,
        contributions,
    }
}
