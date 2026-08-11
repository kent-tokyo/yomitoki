//! Determinism (AGENTS.md §4.5) and atom-order invariance (§14.3, §27).
//!
//! Contract under test: scores, verdict, and the *set* of finding codes are
//! invariant under atom reordering. Raw atom indices inside a finding are
//! **not** compared element-wise — `ring_membership`/`find_ring_families`
//! are positional, so a differently-ordered SMILES for the same molecule
//! legitimately produces different concrete index values. Comparing finding
//! counts/codes (and atom-set sizes, not values) is the correct invariance
//! check; comparing raw indices would be testing the wrong thing.

use std::collections::BTreeMap;
use yomitoki::{AnalysisConfig, FindingCode, SuggestionCode, SynthesizabilityReport};

fn finding_code_multiset(report: &SynthesizabilityReport) -> BTreeMap<FindingCode, usize> {
    let mut counts = BTreeMap::new();
    for finding in &report.findings {
        *counts.entry(finding.code).or_insert(0) += 1;
    }
    counts
}

fn suggestion_code_multiset(report: &SynthesizabilityReport) -> BTreeMap<SuggestionCode, usize> {
    let mut counts = BTreeMap::new();
    for suggestion in &report.suggestions {
        *counts.entry(suggestion.code).or_insert(0) += 1;
    }
    counts
}

// `FindingCode::FunctionalGroupReactive` is one generic code shared by every
// triggered Brenk alert (~105 distinct patterns) — the code multiset alone
// can't tell "epoxide + nitrile" apart from "two epoxides". Comparing
// explanation strings too (each alert's name is embedded in its rendered
// text) restores per-alert discriminating power without needing a code per
// pattern.
fn explanation_multiset(report: &SynthesizabilityReport) -> BTreeMap<String, usize> {
    let mut counts = BTreeMap::new();
    for finding in &report.findings {
        *counts.entry(finding.explanation.clone()).or_insert(0) += 1;
    }
    counts
}

fn assert_equivalent_reports(a: &SynthesizabilityReport, b: &SynthesizabilityReport, label: &str) {
    let eps = 1e-9;
    assert!(
        (a.overall.difficulty.value() - b.overall.difficulty.value()).abs() < eps,
        "{label}: difficulty differs ({} vs {})",
        a.overall.difficulty.value(),
        b.overall.difficulty.value()
    );
    assert!(
        (a.overall.synthesizability.value() - b.overall.synthesizability.value()).abs() < eps,
        "{label}: synthesizability differs"
    );
    assert!(
        (a.overall.confidence.value() - b.overall.confidence.value()).abs() < eps,
        "{label}: confidence differs"
    );
    assert_eq!(
        a.overall.verdict, b.overall.verdict,
        "{label}: verdict differs"
    );
    assert_eq!(
        finding_code_multiset(a),
        finding_code_multiset(b),
        "{label}: finding codes differ"
    );
    assert_eq!(
        explanation_multiset(a),
        explanation_multiset(b),
        "{label}: finding explanations differ"
    );
    assert_eq!(
        a.findings.len(),
        b.findings.len(),
        "{label}: finding count differs"
    );
    assert_eq!(
        suggestion_code_multiset(a),
        suggestion_code_multiset(b),
        "{label}: suggestion codes differ"
    );
}

#[test]
fn atom_reordering_does_not_change_scores_or_findings() {
    let config = AnalysisConfig::default();
    let mol = chematic::smiles::parse("C1CC2CCC1C2").expect("valid SMILES"); // norbornane

    let baseline = yomitoki::analyze(&mol, &config).expect("analyze succeeds");

    for seed in [1u64, 2, 3, 42] {
        let reordered_smiles = chematic::smiles::random_smiles(&mol, seed);
        let reordered_mol = chematic::smiles::parse(&reordered_smiles).unwrap_or_else(|e| {
            panic!("random_smiles produced unparseable output {reordered_smiles:?}: {e}")
        });
        let reordered_report =
            yomitoki::analyze(&reordered_mol, &config).expect("analyze succeeds");
        assert_equivalent_reports(&baseline, &reordered_report, &format!("seed {seed}"));
    }
}

#[test]
fn atom_reordering_does_not_change_functional_group_findings() {
    // Same contract as the norbornane test above, but on a fixture with
    // several *distinct* functional_group_liability alerts (aspirin trips
    // ketone_alpha/phenol/phenolic_aldehyde/active_ester/acetal_ketal) — this
    // is what actually exercises explanation_multiset's discriminating
    // power, since every one of those alerts shares the same
    // FindingCode::FunctionalGroupReactive and would be indistinguishable
    // to a code-only comparison.
    let config = AnalysisConfig::default();
    let mol = chematic::smiles::parse("CC(=O)Oc1ccccc1C(=O)O").expect("aspirin");

    let baseline = yomitoki::analyze(&mol, &config).expect("analyze succeeds");

    for seed in [1u64, 2, 3, 42] {
        let reordered_smiles = chematic::smiles::random_smiles(&mol, seed);
        let reordered_mol = chematic::smiles::parse(&reordered_smiles).unwrap_or_else(|e| {
            panic!("random_smiles produced unparseable output {reordered_smiles:?}: {e}")
        });
        let reordered_report =
            yomitoki::analyze(&reordered_mol, &config).expect("analyze succeeds");
        assert_equivalent_reports(&baseline, &reordered_report, &format!("seed {seed}"));
    }
}

#[test]
fn atom_reordering_does_not_change_multi_family_ring_topology_aggregation() {
    // ring_topology's raw aggregation (round 22 part 10/11) sums squared
    // per-family burdens into a Vec before taking sqrt -- the family
    // *list*'s order depends on chematic's ring discovery, which is
    // itself atom-index-dependent, so a differently-ordered SMILES for
    // the same molecule can hand this Vec its entries in a different
    // order. Floating-point addition is not perfectly associative, so
    // this is a real (if narrow) way a multi-family aggregation could
    // fail to be deterministic that none of the single-ring-family
    // fixtures above (norbornane, aspirin) can exercise -- p-terphenyl
    // (3 independent, unfused ring families) is the first fixture in
    // this suite that actually has more than one family to reorder.
    let config = AnalysisConfig::default();
    let mol = chematic::smiles::parse("c1ccc(-c2ccc(-c3ccccc3)cc2)cc1").expect("p-terphenyl");

    let baseline = yomitoki::analyze(&mol, &config).expect("analyze succeeds");

    for seed in [1u64, 2, 3, 42] {
        let reordered_smiles = chematic::smiles::random_smiles(&mol, seed);
        let reordered_mol = chematic::smiles::parse(&reordered_smiles).unwrap_or_else(|e| {
            panic!("random_smiles produced unparseable output {reordered_smiles:?}: {e}")
        });
        let reordered_report =
            yomitoki::analyze(&reordered_mol, &config).expect("analyze succeeds");
        assert_equivalent_reports(&baseline, &reordered_report, &format!("seed {seed}"));
    }
}

#[test]
fn canonical_vs_original_smiles_produce_the_same_report() {
    let config = AnalysisConfig::default();
    let original = yomitoki::analyze_smiles("C1CC2CCC1C2", &config).expect("valid SMILES");

    let mol = chematic::smiles::parse("C1CC2CCC1C2").expect("valid SMILES");
    let canonical = chematic::smiles::canonical_smiles(&mol);
    let from_canonical = yomitoki::analyze_smiles(&canonical, &config).expect("valid SMILES");

    assert_equivalent_reports(&original, &from_canonical, "canonical vs. original");
}

#[test]
fn repeated_analysis_of_the_same_input_is_bit_identical() {
    let config = AnalysisConfig::default();
    let first = yomitoki::analyze_smiles("CC(=O)Oc1ccccc1C(=O)O", &config).expect("aspirin");
    let second = yomitoki::analyze_smiles("CC(=O)Oc1ccccc1C(=O)O", &config).expect("aspirin");
    assert_eq!(first, second);
}
