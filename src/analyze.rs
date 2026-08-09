//! `analyze`/`analyze_smiles` orchestration and aggregation.
//!
//! Parsing is the only fallible step (AGENTS.md §17): a molecule that
//! parses successfully always yields `Ok(report)`, never `Err`, no matter
//! how difficult or out-of-domain it turns out to be.

use chematic::core::Molecule;

use crate::components::{applicability, ring_topology};
use crate::config::AnalysisConfig;
use crate::error::RenseiError;
use crate::report::{
    ComponentScores, ConfidenceScore, Contribution, Finding, OverallAssessment,
    ProbabilityLikeScore, SynthesizabilityReport, Verdict, finite_or_zero,
};
use crate::rules::{
    DIFFICULTY_CHALLENGING_MAX, DIFFICULTY_LIKELY_ACCESSIBLE_MAX, DIFFICULTY_MODERATE_MAX,
    INDETERMINATE_CONFIDENCE_THRESHOLD,
};

pub fn analyze(
    molecule: &Molecule,
    config: &AnalysisConfig,
) -> Result<SynthesizabilityReport, RenseiError> {
    let applicability_outcome = applicability::compute(molecule, config);
    let ring_outcome = ring_topology::compute(molecule);

    let mut findings: Vec<Finding> = Vec::new();
    findings.extend(applicability_outcome.findings);
    let ring_findings_offset = findings.len();
    findings.extend(ring_outcome.findings);

    // `ComponentScore.findings` were built as offsets into each
    // component's own finding list; rebase the ring-topology component's
    // references onto the combined report-level `findings` Vec.
    let mut ring_score = ring_outcome.score;
    for finding_ref in &mut ring_score.findings {
        finding_ref.0 += ring_findings_offset;
    }

    let difficulty = ring_score.normalized;
    let synthesizability = ProbabilityLikeScore::new(1.0 - difficulty.value());
    let confidence = ConfidenceScore::new(applicability_outcome.score.confidence.value());

    let verdict = if applicability_outcome.out_of_domain {
        Verdict::OutOfDomain
    } else if confidence.value() < INDETERMINATE_CONFIDENCE_THRESHOLD {
        Verdict::Indeterminate
    } else if difficulty.value() < DIFFICULTY_LIKELY_ACCESSIBLE_MAX {
        Verdict::LikelyAccessible
    } else if difficulty.value() < DIFFICULTY_MODERATE_MAX {
        Verdict::ModeratelyAccessible
    } else if difficulty.value() < DIFFICULTY_CHALLENGING_MAX {
        Verdict::Challenging
    } else {
        Verdict::HighlyChallenging
    };

    let dominant_penalties: Vec<Contribution> = {
        let mut ranked: Vec<&Finding> = findings.iter().collect();
        ranked.sort_by(|a, b| {
            b.evidence
                .value
                .unwrap_or(1.0)
                .partial_cmp(&a.evidence.value.unwrap_or(1.0))
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        ranked
            .into_iter()
            .map(|f| Contribution {
                code: f.code,
                name: f.explanation.clone(),
                contribution: ProbabilityLikeScore::new(finite_or_zero(
                    ring_score.contribution.value(),
                )),
            })
            .collect()
    };

    let overall = OverallAssessment {
        synthesizability,
        difficulty,
        confidence,
        verdict,
    };

    let components = ComponentScores {
        size_topology: None,
        ring_topology: Some(ring_score),
        stereochemical_burden: None,
        fragment_rarity: None,
        functional_group_liability: None,
        input_quality: Some(applicability_outcome.score),
    };

    Ok(SynthesizabilityReport {
        overall,
        components,
        findings,
        dominant_penalties,
        dominant_supports: Vec::new(),
        suggestions: Vec::new(),
        applicability: applicability_outcome.report,
        provenance: crate::provenance::build(config),
    })
}

pub fn analyze_smiles(
    smiles: &str,
    config: &AnalysisConfig,
) -> Result<SynthesizabilityReport, RenseiError> {
    let molecule = chematic::smiles::parse(smiles)?;
    analyze(&molecule, config)
}
