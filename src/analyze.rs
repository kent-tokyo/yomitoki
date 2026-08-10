//! `analyze`/`analyze_smiles` orchestration and aggregation.
//!
//! Parsing is the only fallible step (AGENTS.md §17): a molecule that
//! parses successfully always yields `Ok(report)`, never `Err`, no matter
//! how difficult or out-of-domain it turns out to be.

use chematic::core::Molecule;

use crate::components::{
    applicability, fragment_rarity, functional_group_liability, ring_topology, size_topology,
    stereochemical_burden,
};
use crate::config::{AnalysisConfig, Strictness};
use crate::error::YomitokiError;
use crate::report::{
    ComponentScores, ConfidenceScore, Contribution, Finding, OverallAssessment,
    ProbabilityLikeScore, SynthesizabilityReport, Verdict,
};
use crate::rules::{
    AGGREGATE_WEIGHT_FRAGMENT_RARITY, AGGREGATE_WEIGHT_FUNCTIONAL_GROUP_LIABILITY,
    AGGREGATE_WEIGHT_RING_TOPOLOGY, AGGREGATE_WEIGHT_SIZE_TOPOLOGY,
    AGGREGATE_WEIGHT_STEREOCHEMICAL_BURDEN, DIFFICULTY_CHALLENGING_MAX,
    DIFFICULTY_LIKELY_ACCESSIBLE_MAX, DIFFICULTY_MODERATE_MAX, indeterminate_confidence_threshold,
};

/// Analyze an already-parsed molecule. Infallible in practice — `Result` is
/// kept for API-signature symmetry with `analyze_smiles`, but every branch
/// below returns `Ok`; parsing (upstream, in `analyze_smiles`) is the only
/// fallible step (AGENTS.md §17).
pub fn analyze(
    molecule: &Molecule,
    config: &AnalysisConfig,
) -> Result<SynthesizabilityReport, YomitokiError> {
    let applicability_outcome = applicability::compute(molecule, config);
    let ring_outcome = ring_topology::compute(molecule);
    let size_outcome = size_topology::compute(molecule);
    let stereo_outcome = stereochemical_burden::compute(molecule);
    let fg_outcome = functional_group_liability::compute(molecule);
    // Only runs when a corpus is configured — no corpus ships with
    // yomitoki itself (AGENTS.md §5.4), so this is `None` by default and
    // every result below (`ComponentScores.fragment_rarity`, `difficulty`,
    // `dominant_penalties`, `Provenance.model_version`) is unaffected,
    // matching today's behavior exactly when unconfigured.
    let fragment_outcome = config
        .fragment_model
        .corpus
        .as_deref()
        .map(|corpus| fragment_rarity::compute(molecule, corpus));

    let mut findings: Vec<Finding> = Vec::new();
    findings.extend(applicability_outcome.findings);
    let ring_findings_offset = findings.len();
    findings.extend(ring_outcome.findings);
    let size_findings_offset = findings.len();
    findings.extend(size_outcome.findings);
    let stereo_findings_offset = findings.len();
    findings.extend(stereo_outcome.findings);
    let fg_findings_offset = findings.len();
    findings.extend(fg_outcome.findings);
    let fragment_findings_offset = findings.len();
    if let Some(outcome) = &fragment_outcome {
        findings.extend(outcome.findings.clone());
    }

    // `ComponentScore.findings` were built as offsets into each
    // component's own finding list; rebase each difficulty component's
    // references onto the combined report-level `findings` Vec.
    let mut ring_score = ring_outcome.score;
    for finding_ref in &mut ring_score.findings {
        finding_ref.0 += ring_findings_offset;
    }
    let mut size_score = size_outcome.score;
    for finding_ref in &mut size_score.findings {
        finding_ref.0 += size_findings_offset;
    }
    let mut stereo_score = stereo_outcome.score;
    for finding_ref in &mut stereo_score.findings {
        finding_ref.0 += stereo_findings_offset;
    }
    let mut fg_score = fg_outcome.score;
    for finding_ref in &mut fg_score.findings {
        finding_ref.0 += fg_findings_offset;
    }
    let fragment_score = fragment_outcome.as_ref().map(|outcome| {
        let mut score = outcome.score.clone();
        for finding_ref in &mut score.findings {
            finding_ref.0 += fragment_findings_offset;
        }
        score
    });

    let difficulty = ProbabilityLikeScore::new(
        AGGREGATE_WEIGHT_RING_TOPOLOGY * ring_score.normalized.value()
            + AGGREGATE_WEIGHT_SIZE_TOPOLOGY * size_score.normalized.value()
            + AGGREGATE_WEIGHT_STEREOCHEMICAL_BURDEN * stereo_score.normalized.value()
            + AGGREGATE_WEIGHT_FUNCTIONAL_GROUP_LIABILITY * fg_score.normalized.value()
            + fragment_score
                .as_ref()
                .map(|s| AGGREGATE_WEIGHT_FRAGMENT_RARITY * s.normalized.value())
                .unwrap_or(0.0),
    );
    let synthesizability = ProbabilityLikeScore::new(1.0 - difficulty.value());
    let confidence = ConfidenceScore::new(applicability_outcome.score.confidence.value());

    let verdict = select_verdict(
        applicability_outcome.out_of_domain,
        confidence.value(),
        difficulty.value(),
        config.strictness,
    );

    // Ranked by each finding's actual contribution weight across both
    // difficulty-contributing components (not applicability/data-quality
    // findings — AGENTS.md §5.6 forbids conflating score with input
    // quality). Deliberately independent of `Finding.severity`: severity is
    // a per-finding chemistry judgment, contribution is what actually fed
    // `difficulty`. A `Severity::Low` finding can rank above a
    // `Severity::High` one here if its weight is larger (e.g. a very high
    // rotatable-bond count outweighing a bridged ring) — that's the two
    // axes working as designed, not a bug.
    let mut dominant_penalties: Vec<Contribution> = Vec::new();
    dominant_penalties.extend(ring_outcome.contributions);
    dominant_penalties.extend(size_outcome.contributions);
    dominant_penalties.extend(stereo_outcome.contributions);
    dominant_penalties.extend(fg_outcome.contributions);
    if let Some(outcome) = fragment_outcome {
        dominant_penalties.extend(outcome.contributions);
    }
    dominant_penalties.sort_by(|a, b| {
        b.contribution
            .value()
            .partial_cmp(&a.contribution.value())
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    let overall = OverallAssessment {
        synthesizability,
        difficulty,
        confidence,
        verdict,
    };

    let components = ComponentScores {
        size_topology: Some(size_score),
        ring_topology: Some(ring_score),
        stereochemical_burden: Some(stereo_score),
        fragment_rarity: fragment_score,
        functional_group_liability: Some(fg_score),
        input_quality: Some(applicability_outcome.score),
    };

    let suggestions = crate::suggestions::derive(&findings);

    Ok(SynthesizabilityReport {
        overall,
        components,
        findings,
        dominant_penalties,
        dominant_supports: Vec::new(),
        suggestions,
        applicability: applicability_outcome.report,
        provenance: crate::provenance::build(config),
    })
}

/// Parse `smiles` (via chematic) and analyze it. `Err` only if `smiles`
/// fails to parse — a hard-to-synthesize or out-of-domain molecule is
/// never an error (AGENTS.md §17).
pub fn analyze_smiles(
    smiles: &str,
    config: &AnalysisConfig,
) -> Result<SynthesizabilityReport, YomitokiError> {
    let molecule = chematic::smiles::parse(smiles)?;
    analyze(&molecule, config)
}

/// Analyze many molecules with one shared config (AGENTS.md §18). `result[i]`
/// corresponds to `molecules[i]` — input order is preserved regardless of
/// any other molecule's outcome, and one molecule's result never depends on
/// another's (each is an independent `analyze` call), so this is safe to
/// parallelize (e.g. via `rayon`'s `par_iter`) without changing output.
/// Sequential here since nothing in this crate's own benchmarks has shown a
/// need for it yet — AGENTS.md §18 itself only asks that parallelism be
/// *possible* ("Rayon利用はfeature flagでもよい"), not that v0.1 ship it.
pub fn analyze_batch(
    molecules: &[Molecule],
    config: &AnalysisConfig,
) -> Vec<Result<SynthesizabilityReport, YomitokiError>> {
    molecules.iter().map(|m| analyze(m, config)).collect()
}

/// Pure verdict-selection logic, factored out of `analyze` so it's testable
/// without constructing a molecule (AGENTS.md §7: `OutOfDomain` and
/// `Indeterminate` must be genuinely distinct and independently reachable).
fn select_verdict(
    out_of_domain: bool,
    confidence: f64,
    difficulty: f64,
    strictness: Strictness,
) -> Verdict {
    if out_of_domain {
        return Verdict::OutOfDomain;
    }
    if confidence < indeterminate_confidence_threshold(strictness) {
        return Verdict::Indeterminate;
    }
    if difficulty < DIFFICULTY_LIKELY_ACCESSIBLE_MAX {
        Verdict::LikelyAccessible
    } else if difficulty < DIFFICULTY_MODERATE_MAX {
        Verdict::ModeratelyAccessible
    } else if difficulty < DIFFICULTY_CHALLENGING_MAX {
        Verdict::Challenging
    } else {
        Verdict::HighlyChallenging
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // A confidence floor reachable by two of applicability's soft penalties
    // combined (CONFIDENCE_PENALTY_UNUSUAL_VALENCE * CONFIDENCE_PENALTY_STEREO_INCOMPLETE
    // = 0.5 * 0.85 = 0.425, see components/applicability.rs). Standard
    // strictness's threshold (0.45) must sit above this floor or
    // `Indeterminate` can never fire — this test is what would have caught
    // it before shipping.
    const PENALTY_FLOOR_CONFIDENCE: f64 = 0.425;

    // The *lowest* floor: unusual valence combined with a stereo-uncheckable
    // molecule (CONFIDENCE_PENALTY_UNUSUAL_VALENCE * CONFIDENCE_PENALTY_STEREO_UNCHECKABLE
    // = 0.5 * 0.6 = 0.3) — lower than PENALTY_FLOOR_CONFIDENCE above because
    // CONFIDENCE_PENALTY_STEREO_UNCHECKABLE (0.6) is a stronger penalty than
    // CONFIDENCE_PENALTY_STEREO_INCOMPLETE (0.85); the two are mutually
    // exclusive per molecule, so this and PENALTY_FLOOR_CONFIDENCE are never
    // both live at once, but both must independently stay reachable and
    // above the Standard threshold.
    const LOWEST_PENALTY_FLOOR_CONFIDENCE: f64 = 0.3;

    #[test]
    fn out_of_domain_wins_regardless_of_confidence_or_difficulty() {
        assert_eq!(
            select_verdict(true, 1.0, 0.0, Strictness::Standard),
            Verdict::OutOfDomain
        );
    }

    #[test]
    fn indeterminate_is_reachable_at_standard_strictness() {
        assert_eq!(
            select_verdict(false, PENALTY_FLOOR_CONFIDENCE, 0.1, Strictness::Standard),
            Verdict::Indeterminate
        );
    }

    #[test]
    fn indeterminate_is_reachable_at_the_lowest_penalty_floor_too() {
        assert_eq!(
            select_verdict(
                false,
                LOWEST_PENALTY_FLOOR_CONFIDENCE,
                0.1,
                Strictness::Standard
            ),
            Verdict::Indeterminate
        );
    }

    #[test]
    fn lenient_strictness_tolerates_the_penalty_floor() {
        assert_ne!(
            select_verdict(false, PENALTY_FLOOR_CONFIDENCE, 0.1, Strictness::Lenient),
            Verdict::Indeterminate
        );
    }

    #[test]
    fn strict_strictness_is_at_least_as_eager_to_abstain_as_standard() {
        let standard = indeterminate_confidence_threshold(Strictness::Standard);
        let strict = indeterminate_confidence_threshold(Strictness::Strict);
        let lenient = indeterminate_confidence_threshold(Strictness::Lenient);
        assert!(strict >= standard);
        assert!(standard >= lenient);
    }

    #[test]
    fn difficulty_buckets_are_all_reachable() {
        let full_confidence = 1.0;
        assert_eq!(
            select_verdict(false, full_confidence, 0.0, Strictness::Standard),
            Verdict::LikelyAccessible
        );
        assert_eq!(
            select_verdict(false, full_confidence, 0.4, Strictness::Standard),
            Verdict::ModeratelyAccessible
        );
        assert_eq!(
            select_verdict(false, full_confidence, 0.6, Strictness::Standard),
            Verdict::Challenging
        );
        assert_eq!(
            select_verdict(false, full_confidence, 0.9, Strictness::Standard),
            Verdict::HighlyChallenging
        );
    }
}
