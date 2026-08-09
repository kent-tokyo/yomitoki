//! Schema round-trip: serde round-trips, no NaN/Infinity ever hits the
//! wire, and enums serialize as strings (AGENTS.md §16).

use rensei::{
    ApplicabilityReport, AtomIndex, ComponentScore, ComponentScores, ConfidenceScore, Contribution,
    Finding, FindingCode, FindingEvidence, FindingRef, OverallAssessment, ProbabilityLikeScore,
    Provenance, Severity, SynthesizabilityReport, Verdict,
};

fn sample_report() -> SynthesizabilityReport {
    let ring_topology = ComponentScore {
        raw: 0.6,
        normalized: ProbabilityLikeScore::new(0.33),
        confidence: ProbabilityLikeScore::new(1.0),
        contribution: ProbabilityLikeScore::new(0.33),
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
            fragment_rarity: None,
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
        suggestions: Vec::new(),
        applicability: ApplicabilityReport {
            supported_elements: true,
            sanitized: true,
            stereo_complete: true,
            disconnected: false,
            unusual_valence: false,
            domain_distance: None,
        },
        provenance: Provenance {
            schema_version: "0.1.0".to_string(),
            rensei_version: "0.1.0".to_string(),
            chematic_version: "0.11".to_string(),
            ruleset_version: "0.1.0".to_string(),
            config_hash: "sha256:deadbeef".to_string(),
        },
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
    let report = sample_report();
    assert!(report.components.size_topology.is_none());
    assert!(report.components.stereochemical_burden.is_none());
    assert!(report.components.fragment_rarity.is_none());
    assert!(report.components.functional_group_liability.is_none());
}

#[test]
fn scores_from_a_real_analysis_also_round_trip_and_stay_finite() {
    let config = rensei::AnalysisConfig::default();
    let report = rensei::analyze_smiles("C1CC2CCC1C2", &config).expect("valid SMILES");

    let json = serde_json::to_string(&report).expect("serializes");
    assert!(!json.contains("NaN"));
    assert!(!json.contains("Infinity"));

    let back: SynthesizabilityReport = serde_json::from_str(&json).expect("deserializes");
    assert_eq!(report, back);

    assert!((0.0..=1.0).contains(&report.overall.synthesizability.value()));
    assert!((0.0..=1.0).contains(&report.overall.difficulty.value()));
    assert!((0.0..=1.0).contains(&report.overall.confidence.value()));
}
