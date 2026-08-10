//! Schema round-trip: serde round-trips, no NaN/Infinity ever hits the
//! wire, and enums serialize as strings (AGENTS.md §16).

use yomitoki::{
    ApplicabilityReport, AtomIndex, ComponentScore, ComponentScores, ConfidenceScore, Contribution,
    ExpectedEffect, Finding, FindingCode, FindingEvidence, FindingRef, OverallAssessment,
    ProbabilityLikeScore, Provenance, Severity, SimplificationSuggestion, SuggestionCode,
    SynthesizabilityReport, Verdict,
};

fn sample_provenance() -> Provenance {
    Provenance {
        schema_version: "0.1.0".to_string(),
        yomitoki_version: "0.1.0".to_string(),
        chematic_version: "0.11".to_string(),
        ruleset_version: "0.1.0".to_string(),
        fragment_corpus: None,
        config_hash: "sha256:deadbeef".to_string(),
    }
}

fn sample_report() -> SynthesizabilityReport {
    let ring_topology = ComponentScore {
        raw: 0.6,
        normalized: ProbabilityLikeScore::new(0.33),
        confidence: ProbabilityLikeScore::new(1.0),
        contribution: 0.33,
        findings: vec![FindingRef(0)],
    };

    SynthesizabilityReport {
        overall: OverallAssessment {
            synthesizability: ProbabilityLikeScore::new(0.67),
            difficulty: ProbabilityLikeScore::new(0.33),
            confidence: ConfidenceScore::new(1.0),
            verdict: Verdict::ModeratelyAccessible,
        },
        components: ComponentScores {
            size_topology: None,
            ring_topology: Some(ring_topology),
            stereochemical_burden: None,
            functional_group_liability: None,
            input_quality: None,
        },
        findings: vec![Finding {
            code: FindingCode::RingBridgedComplexity,
            severity: Severity::High,
            confidence: ProbabilityLikeScore::new(1.0),
            atoms: vec![AtomIndex(0), AtomIndex(1)],
            evidence: FindingEvidence::default(),
            explanation: "Bridged ring system spanning 7 atoms.".to_string(),
        }],
        dominant_penalties: vec![Contribution {
            code: FindingCode::RingBridgedComplexity,
            name: "Bridged ring system spanning 7 atoms.".to_string(),
            contribution: ProbabilityLikeScore::new(0.33),
        }],
        dominant_supports: Vec::new(),
        fragment_precedent: None,
        suggestions: vec![SimplificationSuggestion {
            code: SuggestionCode::ReplaceBridgedRingWithMonocyclicAnalog,
            target_atoms: vec![AtomIndex(0), AtomIndex(1)],
            rationale: "Bridgehead connectivity drives this ring's contribution.".to_string(),
            expected_effect: ExpectedEffect::MayReduceDifficulty,
            confidence: ProbabilityLikeScore::new(0.5),
        }],
        applicability: ApplicabilityReport {
            supported_elements: true,
            sanitized: true,
            stereo_complete: true,
            stereo_uncheckable: false,
            disconnected: false,
            unusual_valence: false,
            domain_distance: None,
        },
        provenance: sample_provenance(),
    }
}

#[test]
fn round_trips_through_json() {
    let report = sample_report();
    let json = serde_json::to_string(&report).expect("serializes");
    let back: SynthesizabilityReport = serde_json::from_str(&json).expect("deserializes");
    assert_eq!(report, back);
}

#[test]
fn enums_serialize_as_screaming_snake_case_strings() {
    let report = sample_report();
    let json = serde_json::to_string_pretty(&report).expect("serializes");
    assert!(
        json.contains("\"MODERATELY_ACCESSIBLE\""),
        "verdict:\n{json}"
    );
    assert!(
        json.contains("\"RING_BRIDGED_COMPLEXITY\""),
        "finding code:\n{json}"
    );
    assert!(json.contains("\"HIGH\""), "severity:\n{json}");
    assert!(
        json.contains("\"REPLACE_BRIDGED_RING_WITH_MONOCYCLIC_ANALOG\""),
        "suggestion code:\n{json}"
    );
    assert!(
        json.contains("\"MAY_REDUCE_DIFFICULTY\""),
        "expected effect:\n{json}"
    );
}

#[test]
fn no_nan_or_infinity_in_serialized_output() {
    let report = sample_report();
    let json = serde_json::to_string(&report).expect("serializes");
    assert!(!json.contains("NaN"), "output contained NaN:\n{json}");
    assert!(
        !json.contains("Infinity"),
        "output contained Infinity:\n{json}"
    );
}

#[test]
fn unimplemented_components_are_none_not_fabricated_zero() {
    // Against real `analyze` output, not the hand-built fixture above —
    // the fixture is only for exercising the schema shape, and would go
    // stale (falsely claiming components are unimplemented) the moment a
    // real component landed without this test being updated alongside it.
    let config = yomitoki::AnalysisConfig::default();
    let report = yomitoki::analyze_smiles("CCO", &config).expect("valid SMILES");
    assert!(report.components.ring_topology.is_some());
    assert!(report.components.size_topology.is_some());
    assert!(report.components.stereochemical_burden.is_some());
    assert!(report.components.functional_group_liability.is_some());
    assert!(report.components.input_quality.is_some());
    assert!(report.fragment_precedent.is_none());
}

#[test]
fn schema_uses_fragment_precedent_not_fragment_rarity() {
    // Round 18 rename (AGENTS.md §5.4): the serialized field/key must be
    // `fragment_precedent`, and the old `fragment_rarity` name must not
    // appear anywhere in current schema output.
    let report = sample_report();
    let json = serde_json::to_string(&report).expect("serializes");
    assert!(json.contains("\"fragment_precedent\""), "{json}");
    assert!(!json.contains("fragment_rarity"), "{json}");
}

#[test]
fn provenance_carries_fragment_corpus_domain_when_configured() {
    // FragmentCorpusProvenance (round 18) replaces the old bare
    // model_version: Option<String> — round-trips through JSON and
    // exposes the corpus-domain fields a report reader needs to
    // distinguish "rare in this corpus" from "hard to synthesize".
    let mut report = sample_report();
    report.provenance.fragment_corpus = Some(yomitoki::FragmentCorpusProvenance {
        version: "sha256:test".to_string(),
        source_name: "ChEMBL-37".to_string(),
        domain: "bioactivity".to_string(),
        synthesis_focused: false,
        description: "Bioactive compound reference corpus.".to_string(),
        fragment_definition_version: "morgan-ecfp-v1".to_string(),
        reference_distribution_version: "quantile-grid-v1".to_string(),
    });

    let json = serde_json::to_string(&report).expect("serializes");
    let back: SynthesizabilityReport = serde_json::from_str(&json).expect("deserializes");
    assert_eq!(report, back);

    let fragment_corpus = back.provenance.fragment_corpus.expect("configured above");
    assert_eq!(fragment_corpus.source_name, "ChEMBL-37");
    assert_eq!(fragment_corpus.domain, "bioactivity");
    assert!(!fragment_corpus.synthesis_focused);
}

#[test]
fn scores_from_a_real_analysis_also_round_trip_and_stay_finite() {
    let config = yomitoki::AnalysisConfig::default();
    let report = yomitoki::analyze_smiles("C1CC2CCC1C2", &config).expect("valid SMILES");

    let json = serde_json::to_string(&report).expect("serializes");
    assert!(!json.contains("NaN"));
    assert!(!json.contains("Infinity"));

    let back: SynthesizabilityReport = serde_json::from_str(&json).expect("deserializes");
    assert_eq!(report, back);

    assert!((0.0..=1.0).contains(&report.overall.synthesizability.value()));
    assert!((0.0..=1.0).contains(&report.overall.difficulty.value()));
    assert!((0.0..=1.0).contains(&report.overall.confidence.value()));
}
