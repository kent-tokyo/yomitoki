//! Human-readable explanations, generated from structured finding data
//! (AGENTS.md §8.3: structured data is the source of truth, not prose).
//! A message-ID (`FindingCode`) + parameters (`FindingEvidence`, atom count)
//! is already what future localization would need — no i18n crate required
//! to keep that door open.

use crate::report::{FindingCode, FindingEvidence};

pub(crate) fn render(code: FindingCode, evidence: FindingEvidence, atom_count: usize) -> String {
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
        FindingCode::InputUnsupportedElement => format!(
            "Molecule contains {atom_count} atom(s) outside rensei's supported \
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
