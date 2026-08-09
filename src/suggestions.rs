//! Diagnostic simplification suggestions (AGENTS.md §9).
//!
//! v0.1 is diagnostic-only, derived from findings that already exist —
//! no structure is actually rewritten, and no suggestion is presented as a
//! guarantee ("cutting this bond will definitely make it easier to
//! synthesize" is exactly the claim AGENTS.md §9 forbids). Every suggestion
//! below carries [`crate::report::ExpectedEffect::MayReduceDifficulty`],
//! never `LikelyReducesDifficulty` — nothing in v0.1 has been calibrated
//! against real synthesis outcomes, so claiming higher certainty than that
//! would overstate what this crate actually knows.
//!
//! Only 3 of [`crate::report::SuggestionCode`]'s 6 variants are reachable in
//! v0.1, one per finding this module knows how to translate into an
//! actionable suggestion: `RingBridgedComplexity` -> replace-with-monocyclic,
//! `RingMacrocycle` -> simplify-closure, `StereoDensityHigh` -> reduce
//! density. The other 3 (`ReduceAdjacentQuaternaryCenters`,
//! `RemoveSimilarReactiveGroup`, `IncreaseFragmentPrecedent`) have no
//! underlying signal to derive from yet: quaternary-carbon adjacency isn't
//! computed anywhere, `brenk_matches_detailed` unions atoms per pattern
//! rather than reporting per-occurrence matches (so "remove one of several"
//! can't identify which occurrence to point at), and fragment rarity is
//! deferred entirely (§5.4). See `docs/architecture.md`.

use crate::report::{
    ExpectedEffect, Finding, FindingCode, ProbabilityLikeScore, SimplificationSuggestion,
    SuggestionCode,
};
use crate::rules::SUGGESTION_CONFIDENCE_HEURISTIC;

pub(crate) fn derive(findings: &[Finding]) -> Vec<SimplificationSuggestion> {
    findings.iter().filter_map(suggestion_for).collect()
}

fn suggestion_for(finding: &Finding) -> Option<SimplificationSuggestion> {
    let (code, rationale) = match finding.code {
        FindingCode::RingBridgedComplexity => (
            SuggestionCode::ReplaceBridgedRingWithMonocyclicAnalog,
            "Bridgehead connectivity in this ring system is a direct driver of the \
             ring_topology contribution to difficulty. A monocyclic (or less-fused) \
             analog, if the target application allows one, would remove this specific \
             burden — this is a structural heuristic, not a guarantee the replacement \
             is chemically equivalent or that synthesis actually becomes easier."
                .to_string(),
        ),
        FindingCode::RingMacrocycle => (
            SuggestionCode::SimplifyMacrocyclicClosure,
            "Macrocyclic ring closure is a direct driver of the ring_topology \
             contribution to difficulty (large-ring closures often need high-dilution \
             or specialized macrocyclization methods). A smaller ring or acyclic \
             analog, if chemically acceptable, would remove this burden — this is a \
             structural heuristic, not a guarantee."
                .to_string(),
        ),
        FindingCode::StereoDensityHigh => (
            SuggestionCode::ReduceStereocenterDensity,
            "Stereocenters are concentrated in a compact region, leaving little room \
             for staged, orthogonal stereocontrol. Reducing the number of \
             stereocenters, or spreading them further apart in the structure, would \
             lower this contribution to difficulty — this is a structural heuristic, \
             not a guarantee."
                .to_string(),
        ),
        _ => return None,
    };

    Some(SimplificationSuggestion {
        code,
        // `finding.atoms` is already the honest atom set for the finding
        // this suggestion is derived from — empty for StereoDensityHigh,
        // since stereochemical_burden's own findings carry no atom indices
        // (chematic's `stereo_completeness` reports only aggregate counts,
        // not which atoms are centers; `assign_cip`/
        // `tetrahedral_stereo_neighbors` only cover *specified* centers,
        // which would under-count relative to the density this finding is
        // actually about — see docs/architecture.md).
        target_atoms: finding.atoms.clone(),
        rationale,
        expected_effect: ExpectedEffect::MayReduceDifficulty,
        confidence: ProbabilityLikeScore::new(SUGGESTION_CONFIDENCE_HEURISTIC),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::report::{AtomIndex, FindingEvidence, Severity};

    fn finding(code: FindingCode, atoms: Vec<AtomIndex>) -> Finding {
        Finding {
            code,
            severity: Severity::Medium,
            confidence: ProbabilityLikeScore::new(1.0),
            atoms,
            evidence: FindingEvidence::default(),
            explanation: "irrelevant".to_string(),
        }
    }

    #[test]
    fn bridged_ring_finding_produces_a_replace_bridged_ring_suggestion() {
        let findings = vec![finding(
            FindingCode::RingBridgedComplexity,
            vec![AtomIndex(0), AtomIndex(1)],
        )];
        let suggestions = derive(&findings);
        assert_eq!(suggestions.len(), 1);
        assert_eq!(
            suggestions[0].code,
            SuggestionCode::ReplaceBridgedRingWithMonocyclicAnalog
        );
        assert_eq!(
            suggestions[0].target_atoms,
            vec![AtomIndex(0), AtomIndex(1)]
        );
        assert_eq!(
            suggestions[0].expected_effect,
            ExpectedEffect::MayReduceDifficulty
        );
    }

    #[test]
    fn macrocycle_finding_produces_a_simplify_macrocycle_suggestion() {
        let findings = vec![finding(FindingCode::RingMacrocycle, vec![AtomIndex(0)])];
        let suggestions = derive(&findings);
        assert_eq!(suggestions.len(), 1);
        assert_eq!(
            suggestions[0].code,
            SuggestionCode::SimplifyMacrocyclicClosure
        );
    }

    #[test]
    fn stereo_density_finding_produces_a_reduce_density_suggestion_with_no_atoms() {
        // StereoDensityHigh findings never carry atoms in the first place
        // (see components/stereochemical_burden.rs) -- the suggestion must
        // not fabricate target atoms that the underlying finding doesn't have.
        let findings = vec![finding(FindingCode::StereoDensityHigh, Vec::new())];
        let suggestions = derive(&findings);
        assert_eq!(suggestions.len(), 1);
        assert_eq!(
            suggestions[0].code,
            SuggestionCode::ReduceStereocenterDensity
        );
        assert!(suggestions[0].target_atoms.is_empty());
    }

    #[test]
    fn findings_with_no_mapped_suggestion_produce_nothing() {
        let findings = vec![
            finding(FindingCode::StereoCenterCount, vec![]),
            finding(FindingCode::RingSpiro, vec![AtomIndex(0)]),
            finding(FindingCode::FunctionalGroupReactive, vec![AtomIndex(0)]),
        ];
        assert!(derive(&findings).is_empty());
    }

    #[test]
    fn no_findings_produce_no_suggestions() {
        assert!(derive(&[]).is_empty());
    }

    #[test]
    fn every_suggestion_carries_the_named_heuristic_confidence() {
        let findings = vec![
            finding(FindingCode::RingBridgedComplexity, vec![AtomIndex(0)]),
            finding(FindingCode::RingMacrocycle, vec![AtomIndex(1)]),
            finding(FindingCode::StereoDensityHigh, vec![]),
        ];
        for suggestion in derive(&findings) {
            assert_eq!(
                suggestion.confidence.value(),
                SUGGESTION_CONFIDENCE_HEURISTIC
            );
        }
    }
}
