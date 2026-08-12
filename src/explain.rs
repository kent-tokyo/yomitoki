//! Human-readable explanations, generated from structured finding data
//! (AGENTS.md §8.3: structured data is the source of truth, not prose).
//! A message-ID (`FindingCode`) + parameters (`FindingEvidence`, atom count,
//! label) is already what future localization would need — no i18n crate
//! required to keep that door open.

use crate::report::{FindingCode, FindingEvidence};

pub(crate) fn render(
    code: FindingCode,
    evidence: FindingEvidence,
    atom_count: usize,
    label: Option<&str>,
) -> String {
    match code {
        FindingCode::RingBridgedComplexity => format!(
            "Bridged ring system spanning {atom_count} atoms — bridgehead \
             connectivity typically increases structural synthetic difficulty."
        ),
        FindingCode::RingSpiro => format!("Spiro ring junction spanning {atom_count} atoms."),
        FindingCode::RingFusedDense => format!(
            "Densely fused ring system (fusion density {:.2}, above the {:.2} threshold).",
            evidence.value.unwrap_or(0.0),
            evidence.threshold.unwrap_or(0.0)
        ),
        FindingCode::RingMacrocycle => format!(
            "Macrocyclic ring of {} atoms (at or above the {}-atom macrocycle threshold).",
            evidence.value.unwrap_or(0.0) as usize,
            evidence.threshold.unwrap_or(0.0) as usize
        ),
        FindingCode::SizeLargeMolecularWeight => format!(
            "Molecular weight of {:.1} Da is above the {:.0} Da size threshold.",
            evidence.value.unwrap_or(0.0),
            evidence.threshold.unwrap_or(0.0)
        ),
        FindingCode::SizeHighRotatableBondCount => format!(
            "{} rotatable bonds, above the {}-bond threshold.",
            evidence.value.unwrap_or(0.0) as usize,
            evidence.threshold.unwrap_or(0.0) as usize
        ),
        FindingCode::StereoCenterCount => format!(
            "{} tetrahedral stereocenter(s) (specified or unspecified) requiring \
             synthetic control.",
            evidence.value.unwrap_or(0.0) as usize
        ),
        FindingCode::StereoDensityHigh => format!(
            "Stereocenter density {:.2} is above the {:.2} threshold — stereocenters \
             are concentrated in a compact region, leaving little room for staged, \
             orthogonal control.",
            evidence.value.unwrap_or(0.0),
            evidence.threshold.unwrap_or(0.0)
        ),
        FindingCode::StereoAnalysisSkipped => {
            // Not reachable as of chematic 0.13.0 (the one trigger, chematic
            // issue #267, was fixed upstream) -- text kept for schema
            // stability, describing what this code meant while it fired.
            "Stereo analysis could not be run for this molecule: it contained a negatively \
             charged atom, which used to trigger an arithmetic-overflow bug in chematic's \
             stereo perception (panics in debug builds, produces an unverified result in \
             release builds — see chematic issue #267, fixed in chematic 0.13.0). \
             Stereocenter count/density and stereo completeness were unavailable, not \
             verified to be zero/complete."
                .to_string()
        }
        FindingCode::FunctionalGroupReactive => {
            let name = label.unwrap_or("unknown").replace('_', " ");
            if atom_count == 0 {
                // A real Brenk match always has >=1 atom (any SMARTS
                // pattern matches at least one atom) — zero atoms here
                // means the VF2 search was cut off by the visit budget
                // before it could resolve which atoms matched, not a
                // zero-atom match (see brenk_matches_detailed's doc).
                format!(
                    "Possible reactive/unstable functional group: {name} (Brenk et al. 2008 \
                     structural alert) — flagged but not fully resolved before the match \
                     search budget was exhausted."
                )
            } else {
                format!(
                    "Reactive/unstable functional group detected: {name} (Brenk et al. 2008 \
                     structural alert)."
                )
            }
        }
        FindingCode::FunctionalGroupDense => format!(
            "{} distinct functional-group environments (Ertl 2017 clustering), above the \
             {} threshold — multiple independent reactive/functional regions can compete \
             for reagent selectivity and complicate protecting-group strategy.",
            evidence.value.unwrap_or(0.0) as usize,
            evidence.threshold.unwrap_or(0.0) as usize
        ),
        FindingCode::FragmentPrecedentWeak => {
            let pct = evidence.value.unwrap_or(0.0).round() as u32;
            format!(
                "Fragment precedent is weak relative to the configured reference \
                 corpus: {pct}{} percentile — this molecule's structural fragments \
                 are less common than most of the corpus, which may indicate genuine \
                 synthetic novelty, or simply a gap in this corpus's coverage; this \
                 is not a claim about which. fragment_precedent is an explanatory \
                 reference-corpus signal, not a direct synthetic-difficulty term — \
                 it does not affect overall.difficulty.",
                ordinal_suffix(pct)
            )
        }
        FindingCode::FragmentPrecedentStrong => {
            let pct = evidence.value.unwrap_or(0.0).round() as u32;
            format!(
                "Fragment precedent is strong relative to the configured reference \
                 corpus: {pct}{} percentile — this molecule's structural fragments \
                 are more common than most of the corpus. fragment_precedent is an \
                 explanatory reference-corpus signal, not a direct synthetic-difficulty \
                 term — it does not affect overall.difficulty.",
                ordinal_suffix(pct)
            )
        }
        FindingCode::InputUnsupportedElement => format!(
            "Molecule contains {atom_count} atom(s) outside yomitoki's supported \
             element set."
        ),
        FindingCode::InputDisconnected => {
            "Molecule consists of disconnected fragments.".to_string()
        }
        FindingCode::InputUnusualValence => {
            format!("{atom_count} atom(s) have a valence outside normal ranges for their element.")
        }
        FindingCode::InputTooLarge => format!(
            "Molecule has {} heavy atoms, exceeding the configured limit of {}.",
            evidence.value.unwrap_or(0.0) as usize,
            evidence.threshold.unwrap_or(0.0) as usize
        ),
    }
}

/// English ordinal suffix for a percentile display (`92` -> `"nd"`).
fn ordinal_suffix(n: u32) -> &'static str {
    match (n % 100, n % 10) {
        (11..=13, _) => "th",
        (_, 1) => "st",
        (_, 2) => "nd",
        (_, 3) => "rd",
        _ => "th",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn budget_exhausted_functional_group_finding_does_not_claim_full_certainty() {
        // atom_count == 0 is how components/functional_group_liability.rs
        // signals a budget-cut brenk_matches_detailed entry (a real match
        // always has >=1 atom) — this text must read differently from a
        // fully-resolved match, not just omit the atoms.
        let text = render(
            FindingCode::FunctionalGroupReactive,
            FindingEvidence::default(),
            0,
            Some("epoxide"),
        );
        assert!(text.contains("not fully resolved"), "{text}");

        let resolved = render(
            FindingCode::FunctionalGroupReactive,
            FindingEvidence::default(),
            3,
            Some("epoxide"),
        );
        assert!(!resolved.contains("not fully resolved"), "{resolved}");
    }
}
