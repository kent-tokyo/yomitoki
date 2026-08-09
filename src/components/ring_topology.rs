//! Ring topology burden component (AGENTS.md §5.2).

use chematic::core::Molecule;
use chematic::perception::{RingSystemKind, find_ring_families, find_sssr};

use crate::report::{
    AtomIndex, ComponentScore, Finding, FindingCode, FindingEvidence, FindingRef,
    ProbabilityLikeScore, Severity, finite_or_zero,
};
use crate::rules::{
    MACROCYCLE_MIN_RING_SIZE, RING_BURDEN_SCALE, RING_FUSED_DENSITY_FINDING_THRESHOLD,
    RING_WEIGHT_BRIDGED, RING_WEIGHT_FUSED_BASE, RING_WEIGHT_FUSED_DENSITY,
    RING_WEIGHT_MACROCYCLE_BONUS, RING_WEIGHT_SIMPLE, RING_WEIGHT_SPIRO,
};

pub(crate) struct RingTopologyOutcome {
    pub(crate) score: ComponentScore,
    pub(crate) findings: Vec<Finding>,
}

pub(crate) fn compute(mol: &Molecule) -> RingTopologyOutcome {
    let sssr = find_sssr(mol);
    let families = find_ring_families(mol, &sssr);

    let mut findings = Vec::new();
    let mut raw = 0.0;

    for family in &families {
        let atoms: Vec<AtomIndex> = family.atoms.iter().map(|&a| AtomIndex::from(a)).collect();
        let max_ring_size = family
            .ring_indices
            .iter()
            .map(|&i| sssr.rings()[i].len())
            .max()
            .unwrap_or(0);

        let kind_weight = match family.kind {
            RingSystemKind::Simple => RING_WEIGHT_SIMPLE,
            RingSystemKind::Spiro => {
                findings.push(finding(
                    FindingCode::RingSpiro,
                    Severity::Medium,
                    atoms.clone(),
                    FindingEvidence::default(),
                    atoms.len(),
                ));
                RING_WEIGHT_SPIRO
            }
            RingSystemKind::Bridged => {
                findings.push(finding(
                    FindingCode::RingBridgedComplexity,
                    Severity::High,
                    atoms.clone(),
                    FindingEvidence::default(),
                    atoms.len(),
                ));
                RING_WEIGHT_BRIDGED
            }
            RingSystemKind::Fused => {
                // Fusion density: how much the family's rings overlap,
                // relative to the family's atom count. Two disjoint rings
                // sharing zero atoms would give density 0; heavy overlap
                // (many shared atoms per ring) approaches 1.
                let ring_size_sum: usize = family
                    .ring_indices
                    .iter()
                    .map(|&i| sssr.rings()[i].len())
                    .sum();
                let overlap = ring_size_sum.saturating_sub(family.atoms.len());
                let density = if family.atoms.is_empty() {
                    0.0
                } else {
                    overlap as f64 / family.atoms.len() as f64
                };
                if density > RING_FUSED_DENSITY_FINDING_THRESHOLD {
                    findings.push(finding(
                        FindingCode::RingFusedDense,
                        Severity::Medium,
                        atoms.clone(),
                        FindingEvidence {
                            value: Some(density),
                            threshold: Some(RING_FUSED_DENSITY_FINDING_THRESHOLD),
                        },
                        atoms.len(),
                    ));
                }
                RING_WEIGHT_FUSED_BASE + RING_WEIGHT_FUSED_DENSITY * density
            }
        };

        raw += kind_weight;

        if max_ring_size >= MACROCYCLE_MIN_RING_SIZE {
            findings.push(finding(
                FindingCode::RingMacrocycle,
                Severity::Medium,
                atoms.clone(),
                FindingEvidence {
                    value: Some(max_ring_size as f64),
                    threshold: Some(MACROCYCLE_MIN_RING_SIZE as f64),
                },
                atoms.len(),
            ));
            raw += RING_WEIGHT_MACROCYCLE_BONUS;
        }
    }

    let raw = finite_or_zero(raw);
    // Non-linear burden (AGENTS.md §5.1): bounded, monotonic, and saturates
    // rather than being capped or growing unbounded with ring count.
    let normalized = ProbabilityLikeScore::new(1.0 - (-raw / RING_BURDEN_SCALE).exp());

    let score = ComponentScore {
        raw,
        normalized,
        // Ring classification is fully deterministic for any molecule that
        // parsed and passed valence validation — no additional uncertainty
        // to express yet (see docs/architecture.md).
        confidence: ProbabilityLikeScore::new(1.0),
        contribution: normalized,
        findings: (0..findings.len()).map(FindingRef).collect(),
    };

    RingTopologyOutcome { score, findings }
}

fn finding(
    code: FindingCode,
    severity: Severity,
    atoms: Vec<AtomIndex>,
    evidence: FindingEvidence,
    atom_count: usize,
) -> Finding {
    Finding {
        code,
        severity,
        confidence: ProbabilityLikeScore::new(1.0),
        atoms,
        evidence,
        explanation: crate::explain::render(code, evidence, atom_count),
    }
}
