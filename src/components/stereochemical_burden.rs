//! Stereochemical burden component (AGENTS.md §5.3).
//!
//! Deliberately narrow: tetrahedral stereocenter count and density only.
//! Not covered in v0.1, each investigated and left out for a distinct,
//! evidenced reason (round 12 — see `docs/architecture.md`'s Non-goals
//! section for the full detail, this note only summarizes): E/Z
//! double-bond stereo (a real primitive exists — `chematic::chem::cip::
//! assign_cip` assigns E/Z from SMILES `/`/`\` markers, no 2D coordinates
//! needed — but only for explicitly marked bonds, and implementing a
//! specified-only count was rejected as inconsistent with this
//! component's own "specified or unspecified burden equally" policy
//! below); atropisomerism (`chematic::chem::detect_atropisomers` exists
//! but was empirically disqualified — a real determinism bug); contiguous
//! stereocenter runs and quaternary-carbon adjacency (both need an
//! atom-level stereocenter-candidate list chematic keeps private); meso
//! detection (needs graph automorphism, which chematic computes but
//! doesn't expose).
//!
//! Whether a stereocenter was *specified* in the input is an input-quality/
//! confidence concern, handled by the applicability component
//! (`stereo_complete`) — not a difficulty concern. A molecule needs the
//! same synthetic control over an unspecified center as a specified one;
//! only the confidence of *knowing* which configuration was intended
//! differs.
//!
//! A negatively charged atom is a separate, narrower carve-out: chematic's
//! `stereo_completeness` can't safely run on one at all (overflow bug,
//! chematic issue #267, see `components::has_negatively_charged_atom`'s
//! doc) — never a "no stereocenters" claim, always a `StereoAnalysisSkipped`
//! finding plus a lowered component confidence instead.

use chematic::core::Molecule;
use chematic::perception::stereo_validation::stereo_completeness;

use crate::components::has_negatively_charged_atom;
use crate::report::{
    ComponentScore, Contribution, Finding, FindingCode, FindingEvidence, FindingRef,
    ProbabilityLikeScore, Severity, finite_or_zero,
};
use crate::rules::{
    CONFIDENCE_PENALTY_STEREO_UNCHECKABLE, STEREO_BURDEN_SCALE, STEREO_DENSITY_FINDING_THRESHOLD,
    STEREO_WEIGHT_DENSITY, STEREO_WEIGHT_PER_CENTER,
};

pub(crate) struct StereochemicalBurdenOutcome {
    pub(crate) score: ComponentScore,
    pub(crate) findings: Vec<Finding>,
    pub(crate) contributions: Vec<Contribution>,
}

pub(crate) fn compute(mol: &Molecule) -> StereochemicalBurdenOutcome {
    let mut findings = Vec::new();
    let mut contributions = Vec::new();

    // A negatively charged atom overflows chematic's internal Morgan-rank
    // computation (see components::has_negatively_charged_atom's doc,
    // chematic issue #267) — never call stereo_completeness for one.
    // total_centers/density fall back to 0, but that's not a fabricated
    // "no stereocenters" claim: the StereoAnalysisSkipped finding below
    // says why, and the component's own confidence drops accordingly.
    let uncheckable = has_negatively_charged_atom(mol);
    let total_centers = if uncheckable {
        push(
            &mut findings,
            &mut contributions,
            FindingCode::StereoAnalysisSkipped,
            Severity::Medium,
            FindingEvidence::default(),
            0.0,
        );
        0
    } else {
        stereo_completeness(mol).total_centers
    };
    let atom_count = mol.atom_count();
    let density = if atom_count == 0 {
        0.0
    } else {
        total_centers as f64 / atom_count as f64
    };

    let count_weight = finite_or_zero(STEREO_WEIGHT_PER_CENTER * total_centers as f64);
    let density_weight = finite_or_zero(STEREO_WEIGHT_DENSITY * density);

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

    let confidence = if uncheckable {
        // A real, lowered confidence — not the usual 1.0 — since this
        // component's own raw/normalized above are a documented fallback,
        // not a genuine computation (see the StereoAnalysisSkipped finding
        // above). Not wired into `overall.confidence` yet (only
        // applicability's is), but stays honest in the schema regardless.
        CONFIDENCE_PENALTY_STEREO_UNCHECKABLE
    } else {
        // Deterministic descriptor computation (Morgan-rank-based stereocenter
        // detection) — no additional uncertainty to express yet, same
        // rationale as ring_topology/size_topology.
        1.0
    };

    let score = ComponentScore {
        raw,
        normalized,
        confidence: ProbabilityLikeScore::new(confidence),
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
    let explanation = crate::explain::render(code, evidence, 0, None);
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
