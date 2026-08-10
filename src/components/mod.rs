pub(crate) mod applicability;
pub(crate) mod fragment_precedent;
pub(crate) mod functional_group_liability;
pub(crate) mod ring_topology;
pub(crate) mod size_topology;
pub(crate) mod stereochemical_burden;

/// True if any atom carries a negative formal charge.
///
/// Guards every call into chematic's `stereo_completeness` (called by both
/// `applicability` and `stereochemical_burden`): its internal Morgan-rank
/// invariant computation casts `Atom.charge` (`i8`) to `u64` before
/// multiplying, which overflows for any negative charge regardless of
/// magnitude — panics in debug builds, silently produces a corrupted (not
/// obviously wrong, but not trustworthy either) result in release builds.
/// Filed upstream: chematic issue
/// [#267](https://github.com/kent-tokyo/chematic/issues/267). Until fixed,
/// both callers treat a negatively charged atom the same way applicability
/// already treats an unsupported element or unusual valence: a real,
/// named, soft-confidence-penalty condition — never a silent crash, and
/// never a silently fabricated "zero stereocenters" answer.
pub(crate) fn has_negatively_charged_atom(mol: &chematic::core::Molecule) -> bool {
    mol.atoms().any(|(_, atom)| atom.charge < 0)
}
