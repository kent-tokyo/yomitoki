//! Functional-group liability component (AGENTS.md §5.5).
//!
//! Deliberately narrow: reactive/unstable functional groups only, wrapping
//! chematic's Brenk et al. (2008, J. Med. Chem. 51, 5149-5171) structural-
//! alert set directly rather than hand-curating new SMARTS. AGENTS.md §5.5
//! explicitly warns against "chemically weak rules, over-generalized" —
//! reusing an existing, peer-reviewed, already-validated set is the more
//! defensible v0.1 choice. Brenk's set also happens to include
//! `strained_ring_three`/`strained_ring_four`, covering §5.5's "strained
//! motifs" example incidentally, at no extra implementation cost.
//!
//! Not covered in v0.1 (documented, not silently missing): mutually
//! incompatible functional-group combinations, dense functionalization,
//! protecting-group pressure, chemoselectivity burden, polyfunctional
//! symmetry breaking, multiple-similar-reactive-site counting, and
//! difficult oxidation-state combinations — chematic has no oxidation-state
//! API (confirmed absent from `chematic-chem`/`chematic-perception`) and no
//! functional-group-density metric wired in here yet.

use chematic::chem::brenk_matches_detailed;
use chematic::core::Molecule;

use crate::report::{
    AtomIndex, ComponentScore, Contribution, Finding, FindingCode, FindingEvidence, FindingRef,
    ProbabilityLikeScore, Severity, finite_or_zero,
};
use crate::rules::{FG_BURDEN_SCALE, FG_CONFIDENCE_BUDGET_EXHAUSTED, FG_WEIGHT_PER_REACTIVE_GROUP};

pub(crate) struct FunctionalGroupLiabilityOutcome {
    pub(crate) score: ComponentScore,
    pub(crate) findings: Vec<Finding>,
    pub(crate) contributions: Vec<Contribution>,
}

pub(crate) fn compute(mol: &Molecule) -> FunctionalGroupLiabilityOutcome {
    let alerts = brenk_matches_detailed(mol);

    let mut findings = Vec::with_capacity(alerts.len());
    let mut contributions = Vec::with_capacity(alerts.len());

    for (name, atom_indices) in &alerts {
        // An empty atom list means this alert's VF2 enumeration was cut off
        // by the visit budget before completing (see brenk_matches_detailed's
        // doc) — still a real flagged alert, just one whose full match
        // extent is unresolved. Reported at lower per-finding confidence,
        // never dropped silently (AGENTS.md §4.4).
        let budget_exhausted = atom_indices.is_empty();
        let confidence = if budget_exhausted {
            FG_CONFIDENCE_BUDGET_EXHAUSTED
        } else {
            1.0
        };
        let atoms: Vec<AtomIndex> = atom_indices
            .iter()
            .map(|&idx| AtomIndex::from(idx))
            .collect();
        let evidence = FindingEvidence {
            value: Some(atoms.len() as f64),
            threshold: None,
        };
        let explanation = crate::explain::render(
            FindingCode::FunctionalGroupReactive,
            evidence,
            atoms.len(),
            Some(name),
        );
        findings.push(Finding {
            code: FindingCode::FunctionalGroupReactive,
            severity: Severity::Medium,
            confidence: ProbabilityLikeScore::new(confidence),
            atoms,
            evidence,
            explanation: explanation.clone(),
        });
        contributions.push(Contribution {
            code: FindingCode::FunctionalGroupReactive,
            name: explanation,
            contribution: ProbabilityLikeScore::new(finite_or_zero(FG_WEIGHT_PER_REACTIVE_GROUP)),
        });
    }

    let raw = finite_or_zero(FG_WEIGHT_PER_REACTIVE_GROUP * alerts.len() as f64);
    // Non-linear burden (AGENTS.md §5.1), same saturating transform as the
    // other components.
    let normalized = ProbabilityLikeScore::new(1.0 - (-raw / FG_BURDEN_SCALE).exp());

    // Deterministic Brenk pattern matching has no uncertainty of its own;
    // the only source of imprecision is a budget-exhausted alert, so the
    // component's confidence is the minimum across its own findings' (each
    // already carries FG_CONFIDENCE_BUDGET_EXHAUSTED when that happened).
    let confidence = findings
        .iter()
        .map(|f| f.confidence.value())
        .fold(1.0_f64, f64::min);

    let score = ComponentScore {
        raw,
        normalized,
        confidence: ProbabilityLikeScore::new(confidence),
        contribution: normalized,
        findings: (0..findings.len()).map(FindingRef).collect(),
    };

    FunctionalGroupLiabilityOutcome {
        score,
        findings,
        contributions,
    }
}
