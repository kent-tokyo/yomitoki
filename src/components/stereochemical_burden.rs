//! Stereochemical burden component (AGENTS.md §5.3).
//!
//! Deliberately narrow: tetrahedral stereocenter count and density only.
//! Not covered in v0.1 (documented, not silently missing):
//! E/Z double-bond stereo (chematic's E/Z assignment needs 2D coordinates,
//! which the SMILES-only pipeline doesn't have — `assign_ez_from_2d`/
//! `cip_ez_descriptor` both require a `coords: &[(f64, f64)]` argument),
//! atropisomerism, contiguous-run detection, and quaternary-carbon
//! adjacency. AGENTS.md §5.3 explicitly allows a heuristic-only v0.1 slice
//! here ("v0.1で高度なatropisomer判定が困難な場合は、heuristic warningとして実装してよい").
//!
//! Whether a stereocenter was *specified* in the input is an input-quality/
//! confidence concern, handled by the applicability component
//! (`stereo_complete`) — not a difficulty concern. A molecule needs the
//! same synthetic control over an unspecified center as a specified one;
//! only the confidence of *knowing* which configuration was intended
//! differs.

use chematic::core::Molecule;
use chematic::perception::stereo_validation::stereo_completeness;

use crate::report::{
    ComponentScore, Contribution, Finding, FindingCode, FindingEvidence, FindingRef,
    ProbabilityLikeScore, Severity, finite_or_zero,
};
use crate::rules::{
    STEREO_BURDEN_SCALE, STEREO_DENSITY_FINDING_THRESHOLD, STEREO_WEIGHT_DENSITY,
    STEREO_WEIGHT_PER_CENTER,
};

pub(crate) struct StereochemicalBurdenOutcome {
    pub(crate) score: ComponentScore,
    pub(crate) findings: Vec<Finding>,
    pub(crate) contributions: Vec<Contribution>,
}

pub(crate) fn compute(mol: &Molecule) -> StereochemicalBurdenOutcome {
    let completeness = stereo_completeness(mol);
    let total_centers = completeness.total_centers;
    let atom_count = mol.atom_count();
    let density = if atom_count == 0 {
        0.0
    } else {
        total_centers as f64 / atom_count as f64
    };

    let count_weight = finite_or_zero(STEREO_WEIGHT_PER_CENTER * total_centers as f64);
    let density_weight = finite_or_zero(STEREO_WEIGHT_DENSITY * density);

    let mut findings = Vec::new();
    let mut contributions = Vec::new();

    if total_centers > 0 {
        push(
            &mut findings,
            &mut contributions,
            FindingCode::StereoCenterCount,
            Severity::Low,
            FindingEvidence {
                value: Some(total_centers as f64),
                threshold: None,
            },
            count_weight,
        );
    }
    if density > STEREO_DENSITY_FINDING_THRESHOLD {
        push(
            &mut findings,
            &mut contributions,
            FindingCode::StereoDensityHigh,
            Severity::Medium,
            FindingEvidence {
                value: Some(density),
                threshold: Some(STEREO_DENSITY_FINDING_THRESHOLD),
            },
            density_weight,
        );
    }

    let raw = finite_or_zero(count_weight + density_weight);
    // Non-linear burden (AGENTS.md §5.1), same saturating transform as the
    // other components.
    let normalized = ProbabilityLikeScore::new(1.0 - (-raw / STEREO_BURDEN_SCALE).exp());

    let score = ComponentScore {
        raw,
        normalized,
        // Deterministic descriptor computation (Morgan-rank-based stereocenter
        // detection) — no additional uncertainty to express yet, same
        // rationale as ring_topology/size_topology.
        confidence: ProbabilityLikeScore::new(1.0),
        contribution: normalized,
        findings: (0..findings.len()).map(FindingRef).collect(),
    };

    StereochemicalBurdenOutcome {
        score,
        findings,
        contributions,
    }
}

fn push(
    findings: &mut Vec<Finding>,
    contributions: &mut Vec<Contribution>,
    code: FindingCode,
    severity: Severity,
    evidence: FindingEvidence,
    weight: f64,
) {
    let explanation = crate::explain::render(code, evidence, 0);
    findings.push(Finding {
        code,
        severity,
        confidence: ProbabilityLikeScore::new(1.0),
        atoms: Vec::new(),
        evidence,
        explanation: explanation.clone(),
    });
    contributions.push(Contribution {
        code,
        name: explanation,
        contribution: ProbabilityLikeScore::new(finite_or_zero(weight)),
    });
}
