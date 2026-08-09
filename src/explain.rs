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
             connectivity typically increases synthetic difficulty."
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
