//! `YomitokiError` (AGENTS.md §17): parsing is the only fallible step, and
//! it must actually be an `Err`, not a panic, at the library level — the
//! CLI's own invalid-input handling (`tests/cli.rs`) exercises the binary,
//! not `analyze_smiles` directly, so this was previously untested here.

use yomitoki::{AnalysisConfig, YomitokiError, analyze_smiles};

#[test]
fn invalid_smiles_is_a_parse_error_not_a_panic() {
    let config = AnalysisConfig::default();
    let result = analyze_smiles("this is not valid smiles!!!", &config);
    assert!(matches!(result, Err(YomitokiError::ParseError(_))));
}

#[test]
fn empty_smiles_is_a_parse_error() {
    let config = AnalysisConfig::default();
    let result = analyze_smiles("", &config);
    assert!(matches!(result, Err(YomitokiError::ParseError(_))));
}

#[test]
fn parse_error_display_mentions_the_failure() {
    let config = AnalysisConfig::default();
    let err = analyze_smiles("(((", &config).expect_err("unbalanced parens must not parse");
    let message = err.to_string();
    assert!(
        message.contains("failed to parse molecule"),
        "message: {message}"
    );
}
