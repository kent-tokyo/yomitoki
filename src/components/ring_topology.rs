//! Ring topology burden component (AGENTS.md §5.2).

use chematic::core::Molecule;
use chematic::perception::{RingSystemKind, find_ring_families, find_sssr};

use crate::report::{
    AtomIndex, ComponentScore, Contribution, Finding, FindingCode, FindingEvidence, FindingRef,
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
    /// One entry per finding in `findings` (same order), carrying the
    /// *actual* weight that finding contributed to `raw` — not a shared
    /// component-wide number. This is what `dominant_penalties` ranks by.
    pub(crate) contributions: Vec<Contribution>,
}

pub(crate) fn compute(mol: &Molecule) -> RingTopologyOutcome {
    let sssr = find_sssr(mol);
    let families = find_ring_families(mol, &sssr);

    let mut findings = Vec::new();
    let mut contributions = Vec::new();
    // One entry per ring family, its own kind-weight plus its own
    // macrocycle bonus if it qualifies -- aggregated below via L2 norm
    // rather than a plain sum (round 22 part 10, branch
    // `experiment/ring-topology-family-multiplicity`: linear (L1) summing
    // across *separate* families let several ordinary, independent
    // aromatic rings accumulate burden by count alone, which MPScore's
    // complex-ring stratum showed skews chemist-easy at every fixed total
    // ring count -- e.g. mean family count 1.11 (hard) vs. 3.16 (easy) at
    // 5 total rings. L2 keeps a single genuinely complex family's burden
    // close to intact while damping pure multiplicity of ordinary ones,
    // and -- unlike a plain sum-cap or max() -- still grows (not frozen)
    // as more, comparably-sized families are added, and never *decreases*
    // versus fewer families (sqrt(x²+y²) ≥ x for y ≥ 0), preserving the
    // "more ring complexity never lowers burden" invariant tested in
    // `tests/ring_topology.rs`.
    let mut family_burdens: Vec<f64> = Vec::new();

    let push = |findings: &mut Vec<Finding>,
                contributions: &mut Vec<Contribution>,
                code: FindingCode,
                severity: Severity,
                atoms: Vec<AtomIndex>,
                evidence: FindingEvidence,
                weight: f64| {
        let atom_count = atoms.len();
        let explanation = crate::explain::render(code, evidence, atom_count, None);
        findings.push(Finding {
            code,
            severity,
            confidence: ProbabilityLikeScore::new(1.0),
            atoms,
            evidence,
            explanation: explanation.clone(),
        });
        contributions.push(Contribution {
            code,
            name: explanation,
            contribution: ProbabilityLikeScore::new(finite_or_zero(weight)),
        });
    };

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
                push(
                    &mut findings,
                    &mut contributions,
                    FindingCode::RingSpiro,
                    Severity::Medium,
                    atoms.clone(),
                    FindingEvidence::default(),
                    RING_WEIGHT_SPIRO,
                );
                RING_WEIGHT_SPIRO
            }
            RingSystemKind::Bridged => {
                push(
                    &mut findings,
                    &mut contributions,
                    FindingCode::RingBridgedComplexity,
                    Severity::High,
                    atoms.clone(),
                    FindingEvidence::default(),
                    RING_WEIGHT_BRIDGED,
                );
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
                let weight = RING_WEIGHT_FUSED_BASE + RING_WEIGHT_FUSED_DENSITY * density;
                if density > RING_FUSED_DENSITY_FINDING_THRESHOLD {
                    push(
                        &mut findings,
                        &mut contributions,
                        FindingCode::RingFusedDense,
                        Severity::Medium,
                        atoms.clone(),
                        FindingEvidence {
                            value: Some(density),
                            threshold: Some(RING_FUSED_DENSITY_FINDING_THRESHOLD),
                        },
                        weight,
                    );
                }
                weight
            }
        };

        let mut family_burden = kind_weight;

        if max_ring_size >= MACROCYCLE_MIN_RING_SIZE {
            push(
                &mut findings,
                &mut contributions,
                FindingCode::RingMacrocycle,
                Severity::Medium,
                atoms.clone(),
                FindingEvidence {
                    value: Some(max_ring_size as f64),
                    threshold: Some(MACROCYCLE_MIN_RING_SIZE as f64),
                },
                RING_WEIGHT_MACROCYCLE_BONUS,
            );
            family_burden += RING_WEIGHT_MACROCYCLE_BONUS;
        }

        family_burdens.push(family_burden);
    }

    let raw = finite_or_zero(family_burdens.iter().map(|f| f * f).sum::<f64>().sqrt());
    // Non-linear burden (AGENTS.md §5.1): bounded, monotonic, and saturates
    // rather than being capped or growing unbounded with ring count.
    let normalized = ProbabilityLikeScore::new(1.0 - (-raw / RING_BURDEN_SCALE).exp());

    // Rank by actual per-finding weight, highest first — this is what
    // `SynthesizabilityReport::dominant_penalties` surfaces (AGENTS.md
    // §4.1, §15). Ordering must reflect the weights that fed `raw`, not a
    // single component-wide number repeated across every finding.
    let mut ranked: Vec<(Finding, Contribution)> =
        findings.into_iter().zip(contributions).collect();
    ranked.sort_by(|(_, a), (_, b)| {
        b.contribution
            .value()
            .partial_cmp(&a.contribution.value())
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    let (findings, contributions): (Vec<Finding>, Vec<Contribution>) = ranked.into_iter().unzip();

    let score = ComponentScore {
        raw,
        normalized,
        // Ring classification is fully deterministic for any molecule that
        // parsed and passed valence validation — no additional uncertainty
        // to express yet (see docs/architecture.md).
        confidence: ProbabilityLikeScore::new(1.0),
        contribution: normalized.value(),
        findings: (0..findings.len()).map(FindingRef).collect(),
    };

    RingTopologyOutcome {
        score,
        findings,
        contributions,
    }
}
