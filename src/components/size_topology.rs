//! Size / topology burden component (AGENTS.md §5.1).
//!
//! Molecular weight, rotatable-bond count, and heteroatom count — see
//! [`crate::rules::SIZE_WEIGHT_PER_HETEROATOM`] for the heteroatom term's
//! full semantic contract and evidence. Ring-shape complexity
//! (fused/bridged/spiro/macrocycle) is `ring_topology`'s job, not this
//! component's — see `docs/architecture.md` for why the two stay separate
//! rather than one combined "size" burden.
//!
//! No `Finding`/`FindingCode` is emitted for elevated heteroatom count —
//! deliberately: a structured Finding is a separate, explanation-surface
//! change from the scoring axis itself, not added here.

use chematic::chem::{molecular_weight, num_heteroatoms, rotatable_bond_count};
use chematic::core::Molecule;

use crate::report::{
    ComponentScore, Contribution, Finding, FindingCode, FindingEvidence, FindingRef,
    ProbabilityLikeScore, Severity, finite_or_zero,
};
use crate::rules::{
    SIZE_BURDEN_SCALE, SIZE_HIGH_ROTATABLE_BOND_THRESHOLD, SIZE_LARGE_MOLECULAR_WEIGHT_THRESHOLD,
    SIZE_WEIGHT_PER_HETEROATOM, SIZE_WEIGHT_PER_MOLECULAR_WEIGHT_UNIT,
    SIZE_WEIGHT_PER_ROTATABLE_BOND,
};

pub(crate) struct SizeTopologyOutcome {
    pub(crate) score: ComponentScore,
    pub(crate) findings: Vec<Finding>,
    /// One entry per finding in `findings`, carrying that finding's actual
    /// weight — same "don't share one number across findings" discipline
    /// `ring_topology` uses, for the same `dominant_penalties` reason.
    pub(crate) contributions: Vec<Contribution>,
}

pub(crate) fn compute(mol: &Molecule) -> SizeTopologyOutcome {
    let mw = molecular_weight(mol);
    let rotatable_bonds = rotatable_bond_count(mol);
    let heteroatoms = num_heteroatoms(mol);

    let mw_weight = finite_or_zero(SIZE_WEIGHT_PER_MOLECULAR_WEIGHT_UNIT * mw);
    let rotatable_weight = finite_or_zero(SIZE_WEIGHT_PER_ROTATABLE_BOND * rotatable_bonds as f64);
    let heteroatom_weight = finite_or_zero(SIZE_WEIGHT_PER_HETEROATOM * heteroatoms as f64);
    let raw = finite_or_zero(mw_weight + rotatable_weight + heteroatom_weight);

    let mut findings = Vec::new();
    let mut contributions = Vec::new();

    if mw > SIZE_LARGE_MOLECULAR_WEIGHT_THRESHOLD {
        push(
            &mut findings,
            &mut contributions,
            FindingCode::SizeLargeMolecularWeight,
            Severity::Low,
            FindingEvidence {
                value: Some(mw),
                threshold: Some(SIZE_LARGE_MOLECULAR_WEIGHT_THRESHOLD),
            },
            mw_weight,
        );
    }
    if rotatable_bonds > SIZE_HIGH_ROTATABLE_BOND_THRESHOLD {
        push(
            &mut findings,
            &mut contributions,
            FindingCode::SizeHighRotatableBondCount,
            Severity::Low,
            FindingEvidence {
                value: Some(rotatable_bonds as f64),
                threshold: Some(SIZE_HIGH_ROTATABLE_BOND_THRESHOLD as f64),
            },
            rotatable_weight,
        );
    }

    // Non-linear burden (AGENTS.md §5.1), same saturating transform as
    // ring_topology.
    let normalized = ProbabilityLikeScore::new(1.0 - (-raw / SIZE_BURDEN_SCALE).exp());

    let score = ComponentScore {
        raw,
        normalized,
        // Both inputs are plain deterministic descriptor calls — no
        // additional uncertainty to express (same rationale as
        // ring_topology's fixed confidence).
        confidence: ProbabilityLikeScore::new(1.0),
        contribution: normalized.value(),
        findings: (0..findings.len()).map(FindingRef).collect(),
    };

    SizeTopologyOutcome {
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
