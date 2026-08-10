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
//! 3 of [`crate::report::SuggestionCode`]'s 6 variants are reachable in
//! v0.1, one per finding this module knows how to translate into an
//! actionable suggestion: `RingBridgedComplexity` -> replace-with-monocyclic,
//! `RingMacrocycle` -> simplify-closure, `StereoDensityHigh` -> reduce
//! density. The other 3 are unreachable, each for a different reason (see
//! `docs/architecture.md`):
//! `IncreaseFragmentPrecedent` — reachable through round 20, removed round
//! 21 (option C): once `fragment_precedent` stopped contributing to
//! `overall.difficulty`, "this would lower this contribution to
//! difficulty" became a false claim, so the suggestion was retired rather
//! than kept and made misleading.
//! `ReduceAdjacentQuaternaryCenters`/`RemoveSimilarReactiveGroup` have no
//! underlying signal to derive from yet: quaternary-carbon adjacency isn't
//! computed anywhere, and `brenk_matches_detailed` unions atoms per pattern
//! rather than reporting per-occurrence matches (so "remove one of several"
//! can't identify which occurrence to point at).

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
        // FindingCode::FragmentPrecedentWeak intentionally has no arm here
        // since round 21 (option C): fragment_precedent no longer
        // contributes to overall.difficulty, so "a more precedented analog
        // would lower this contribution to difficulty" would be false --
        // SuggestionCode::IncreaseFragmentPrecedent is kept in the schema
        // (non_exhaustive) but is permanently unreachable, same treatment
        // as ReduceAdjacentQuaternaryCenters/RemoveSimilarReactiveGroup
        // below.
        _ => return None,
    };

    Some(SimplificationSuggestion {
        code,
        // `finding.atoms` is already the honest atom set for the finding
        // this suggestion is derived from. For StereoDensityHigh this used
        // to always be empty (chematic's `stereo_completeness` reported
        // only aggregate counts, not which atoms are centers) -- chematic
        // 0.13.0's `stereo_centers` API (issue #263) gives real per-atom
        // indices instead, specified or unspecified alike, matching this
        // component's own "burden equally" policy (see
        // components/stereochemical_burden.rs and docs/architecture.md).
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
    fn stereo_density_finding_produces_a_reduce_density_suggestion() {
        // As of chematic 0.13.0's `stereo_centers` API (issue #263),
        // StereoDensityHigh findings carry real atom indices (see
        // components/stereochemical_burden.rs) -- `derive` must pass them
        // through honestly rather than dropping them.
        let findings = vec![finding(
            FindingCode::StereoDensityHigh,
            vec![AtomIndex(2), AtomIndex(5)],
        )];
        let suggestions = derive(&findings);
        assert_eq!(suggestions.len(), 1);
        assert_eq!(
            suggestions[0].code,
            SuggestionCode::ReduceStereocenterDensity
        );
        assert_eq!(
            suggestions[0].target_atoms,
            vec![AtomIndex(2), AtomIndex(5)]
        );
    }

    #[test]
    fn stereo_density_finding_with_no_atoms_does_not_fabricate_any() {
        // Not a real case as of chematic 0.13.0 (stereo_centers always
        // populates atoms when total_centers > 0, and StereoDensityHigh
        // only fires when centers exist), but `derive` itself has no way
        // to know that -- it must not invent atoms the input finding
        // didn't have, whatever the source turns out to be.
        let findings = vec![finding(FindingCode::StereoDensityHigh, Vec::new())];
        let suggestions = derive(&findings);
        assert_eq!(suggestions.len(), 1);
        assert!(suggestions[0].target_atoms.is_empty());
    }

    #[test]
    fn fragment_precedent_weak_finding_produces_no_suggestion() {
        // Round 21 (option C): IncreaseFragmentPrecedent was retired --
        // fragment_precedent no longer contributes to overall.difficulty,
        // so "this would lower this contribution to difficulty" would be
        // a false claim. See suggestions.rs's module doc.
        let findings = vec![finding(FindingCode::FragmentPrecedentWeak, Vec::new())];
        assert!(derive(&findings).is_empty());
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
