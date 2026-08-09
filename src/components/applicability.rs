//! Input quality / applicability component (AGENTS.md §5.6).

use chematic::core::{Molecule, validate_valence};
use chematic::perception::stereo_validation::stereo_completeness;

use crate::config::AnalysisConfig;
use crate::report::{
    ApplicabilityReport, AtomIndex, ComponentScore, Finding, FindingCode, FindingEvidence,
    FindingRef, ProbabilityLikeScore, Severity, finite_or_zero,
};
use crate::rules::{
    CONFIDENCE_PENALTY_STEREO_INCOMPLETE, CONFIDENCE_PENALTY_UNUSUAL_VALENCE, SUPPORTED_ELEMENTS,
};

/// Result of the applicability component: the report-facing summary, the
/// aggregation-facing score, any findings raised, and whether a hard
/// out-of-domain trigger fired (consumed by `analyze.rs` for the verdict).
pub(crate) struct ApplicabilityOutcome {
    pub(crate) report: ApplicabilityReport,
    pub(crate) score: ComponentScore,
    pub(crate) findings: Vec<Finding>,
    pub(crate) out_of_domain: bool,
}

pub(crate) fn compute(mol: &Molecule, config: &AnalysisConfig) -> ApplicabilityOutcome {
    let mut findings = Vec::new();

    let unsupported_atoms: Vec<AtomIndex> = mol
        .atoms()
        .filter(|(_, atom)| !SUPPORTED_ELEMENTS.contains(&atom.element))
        .map(|(idx, _)| AtomIndex::from(idx))
        .collect();
    let supported_elements = unsupported_atoms.is_empty();
    if !supported_elements {
        findings.push(Finding {
            code: FindingCode::InputUnsupportedElement,
            severity: Severity::High,
            confidence: ProbabilityLikeScore::new(1.0),
            atoms: unsupported_atoms.clone(),
            evidence: FindingEvidence::default(),
            explanation: crate::explain::render(
                FindingCode::InputUnsupportedElement,
                FindingEvidence::default(),
                unsupported_atoms.len(),
                None,
            ),
        });
    }

    let valence_errors = validate_valence(mol);
    let unusual_valence = !valence_errors.is_empty();
    if unusual_valence {
        let atoms: Vec<AtomIndex> = valence_errors
            .iter()
            .map(|e| AtomIndex::from(e.atom))
            .collect();
        findings.push(Finding {
            code: FindingCode::InputUnusualValence,
            severity: Severity::Medium,
            confidence: ProbabilityLikeScore::new(1.0),
            atoms: atoms.clone(),
            evidence: FindingEvidence::default(),
            explanation: crate::explain::render(
                FindingCode::InputUnusualValence,
                FindingEvidence::default(),
                atoms.len(),
                None,
            ),
        });
    }

    let disconnected = !mol.is_connected();
    if disconnected {
        findings.push(Finding {
            code: FindingCode::InputDisconnected,
            severity: Severity::High,
            confidence: ProbabilityLikeScore::new(1.0),
            atoms: Vec::new(),
            evidence: FindingEvidence::default(),
            explanation: crate::explain::render(
                FindingCode::InputDisconnected,
                FindingEvidence::default(),
                0,
                None,
            ),
        });
    }

    let stereo = stereo_completeness(mol);
    let stereo_complete = stereo.unspecified == 0;

    let atom_count = mol.atom_count();
    let too_large = atom_count > config.max_heavy_atoms;
    if too_large {
        findings.push(Finding {
            code: FindingCode::InputTooLarge,
            severity: Severity::High,
            confidence: ProbabilityLikeScore::new(1.0),
            atoms: Vec::new(),
            evidence: FindingEvidence {
                value: Some(atom_count as f64),
                threshold: Some(config.max_heavy_atoms as f64),
            },
            explanation: crate::explain::render(
                FindingCode::InputTooLarge,
                FindingEvidence {
                    value: Some(atom_count as f64),
                    threshold: Some(config.max_heavy_atoms as f64),
                },
                0,
                None,
            ),
        });
    }

    // Hard out-of-domain triggers: structural conditions AGENTS.md §28
    // explicitly places outside v0.1's scope (full element coverage,
    // arbitrary size) or that make any score meaningless (disconnected
    // input isn't one molecule). Valence irregularities and incomplete
    // stereo are handled as soft confidence penalties below instead —
    // they're common in legitimate input, not structural impossibilities.
    let out_of_domain = !supported_elements || disconnected || too_large;

    let mut confidence = 1.0;
    if unusual_valence {
        confidence *= CONFIDENCE_PENALTY_UNUSUAL_VALENCE;
    }
    if !stereo_complete {
        confidence *= CONFIDENCE_PENALTY_STEREO_INCOMPLETE;
    }
    let confidence = ProbabilityLikeScore::new(finite_or_zero(confidence));

    let report = ApplicabilityReport {
        supported_elements,
        sanitized: !unusual_valence,
        stereo_complete,
        disconnected,
        unusual_valence,
        domain_distance: None,
    };

    let score = ComponentScore {
        raw: finite_or_zero(1.0 - confidence.value()),
        normalized: ProbabilityLikeScore::new(1.0 - confidence.value()),
        confidence,
        contribution: ProbabilityLikeScore::new(0.0),
        findings: (0..findings.len()).map(FindingRef).collect(),
    };

    ApplicabilityOutcome {
        report,
        score,
        findings,
        out_of_domain,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::report::FindingCode;

    fn mol(smiles: &str) -> Molecule {
        chematic::smiles::parse(smiles).expect("valid SMILES")
    }

    #[test]
    fn disconnected_fragments_trigger_out_of_domain() {
        let outcome = compute(&mol("CCO.CCO"), &AnalysisConfig::default());
        assert!(outcome.report.disconnected);
        assert!(outcome.out_of_domain);
        assert!(
            outcome
                .findings
                .iter()
                .any(|f| f.code == FindingCode::InputDisconnected)
        );
    }

    #[test]
    fn unsupported_element_triggers_out_of_domain() {
        let outcome = compute(&mol("C[Se]C"), &AnalysisConfig::default());
        assert!(!outcome.report.supported_elements);
        assert!(outcome.out_of_domain);
        assert!(
            outcome
                .findings
                .iter()
                .any(|f| f.code == FindingCode::InputUnsupportedElement)
        );
    }

    #[test]
    fn unspecified_stereocenter_lowers_confidence_but_stays_in_domain() {
        // Alanine without stereo annotation: one unspecified tetrahedral
        // center, otherwise unremarkable.
        let outcome = compute(&mol("CC(N)C(=O)O"), &AnalysisConfig::default());
        assert!(!outcome.report.stereo_complete);
        assert!(!outcome.out_of_domain);
        assert!(
            (outcome.score.confidence.value() - CONFIDENCE_PENALTY_STEREO_INCOMPLETE).abs() < 1e-9
        );
    }

    #[test]
    fn unusual_valence_lowers_confidence_but_stays_in_domain() {
        let outcome = compute(&mol("CC(C)(C)(C)C"), &AnalysisConfig::default());
        assert!(outcome.report.unusual_valence);
        assert!(!outcome.out_of_domain);
        assert!(
            (outcome.score.confidence.value() - CONFIDENCE_PENALTY_UNUSUAL_VALENCE).abs() < 1e-9
        );
    }

    #[test]
    fn combined_valence_and_stereo_penalties_multiply() {
        // Both a 5-bonded carbon (unusual valence) and an unspecified
        // stereocenter (F, Cl, Br, H substituents) in one connected,
        // fully-supported-element molecule.
        let outcome = compute(&mol("FC(Cl)(Br)C(C)(C)(C)C"), &AnalysisConfig::default());
        assert!(outcome.report.unusual_valence);
        assert!(!outcome.report.stereo_complete);
        assert!(!outcome.out_of_domain);
        let expected = CONFIDENCE_PENALTY_UNUSUAL_VALENCE * CONFIDENCE_PENALTY_STEREO_INCOMPLETE;
        assert!(
            (outcome.score.confidence.value() - expected).abs() < 1e-9,
            "confidence={} expected={expected}",
            outcome.score.confidence.value()
        );
    }

    #[test]
    fn extreme_size_triggers_out_of_domain() {
        let config = AnalysisConfig {
            max_heavy_atoms: 2,
            ..AnalysisConfig::default()
        };
        let outcome = compute(&mol("CCO"), &config);
        assert!(outcome.out_of_domain);
        assert!(
            outcome
                .findings
                .iter()
                .any(|f| f.code == FindingCode::InputTooLarge)
        );
    }

    #[test]
    fn clean_molecule_has_full_confidence_and_stays_in_domain() {
        let outcome = compute(&mol("CCO"), &AnalysisConfig::default());
        assert!(!outcome.out_of_domain);
        assert_eq!(outcome.score.confidence.value(), 1.0);
        assert!(outcome.findings.is_empty());
    }
}
