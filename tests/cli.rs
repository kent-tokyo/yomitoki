//! End-to-end tests of the `yomitoki` binary (AGENTS.md §15), run as a
//! separate process via `Command` — this is the only way to exercise the
//! real arg parsing, exit codes, and file I/O together (unit tests inside
//! `src/bin/yomitoki.rs` cover `parse_args`/`render_human` in isolation).

use std::io::Write;
use std::process::Command;
use tempfile::NamedTempFile;

fn yomitoki() -> Command {
    Command::new(env!("CARGO_BIN_EXE_yomitoki"))
}

#[test]
fn single_molecule_human_output_and_success_exit_code() {
    let output = yomitoki()
        .args(["analyze", "C1CC2CCC1C2"])
        .output()
        .expect("run yomitoki");
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("utf8");
    assert!(stdout.starts_with("Verdict: ModeratelyAccessible"));
    assert!(stdout.contains("Dominant penalties:"));
    assert!(stdout.contains("Simplification suggestions"));
}

#[test]
fn invalid_smiles_exits_nonzero_and_reports_to_stderr() {
    let output = yomitoki()
        .args(["analyze", "not_a_smiles((("])
        .output()
        .expect("run yomitoki");
    assert!(!output.status.success());
    assert!(output.stdout.is_empty());
    let stderr = String::from_utf8(output.stderr).expect("utf8");
    assert!(stderr.contains("error:"));
}

#[test]
fn no_arguments_prints_usage_and_succeeds() {
    let output = yomitoki().output().expect("run yomitoki");
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("utf8");
    assert!(stdout.contains("Usage:"));
}

#[test]
fn missing_smiles_or_input_is_a_usage_error() {
    let output = yomitoki().args(["analyze"]).output().expect("run yomitoki");
    assert_eq!(output.status.code(), Some(2));
}

#[test]
fn fragment_corpus_flag_with_a_missing_directory_fails_cleanly() {
    let output = yomitoki()
        .args([
            "analyze",
            "CCO",
            "--fragment-corpus",
            "/nonexistent/path/that/does/not/exist",
        ])
        .output()
        .expect("run yomitoki");
    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).expect("utf8");
    assert!(stderr.contains("--fragment-corpus"), "{stderr}");
}

#[test]
fn fragment_corpus_flag_with_a_real_corpus_populates_fragment_rarity() {
    // Same fixture-building approach as tests/fragment_rarity.rs: hashes
    // come from the real chematic::fp::morgan_fp_counts output for "CCO",
    // not hand-computed, so they're guaranteed to match what the running
    // binary itself will compute.
    let dir = tempfile::tempdir().expect("tempdir");
    let mol = chematic::smiles::parse("CCO").expect("valid fixture SMILES");
    let counts = chematic::fp::morgan_fp_counts(&mol, 2);
    let fragments: Vec<serde_json::Value> = counts
        .keys()
        .map(|hash| serde_json::json!({"radius": 2, "fragment_hash": hash, "occurrence_count": 1}))
        .collect();
    let table = serde_json::json!({
        "total_molecules_processed": 1,
        "distinct_fragment_count": fragments.len(),
        "fragments": fragments,
    });
    std::fs::write(
        dir.path().join("fragment_frequencies.json"),
        serde_json::to_vec(&table).unwrap(),
    )
    .unwrap();
    std::fs::write(
        dir.path().join("manifest.json"),
        serde_json::to_vec(&serde_json::json!({"artifact_sha256": "sha256:cli-test-fixture"}))
            .unwrap(),
    )
    .unwrap();

    let output = yomitoki()
        .args(["analyze", "CCO", "--format", "json"])
        .arg("--fragment-corpus")
        .arg(dir.path())
        .output()
        .expect("run yomitoki");
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("utf8");
    let report: serde_json::Value = serde_json::from_str(&stdout).expect("valid json");
    assert!(
        !report["components"]["fragment_rarity"].is_null(),
        "{stdout}"
    );
    assert_eq!(
        report["provenance"]["model_version"],
        "sha256:cli-test-fixture"
    );
}

#[test]
fn batch_mode_preserves_order_and_continues_past_a_bad_record() {
    let mut input = NamedTempFile::new().expect("create temp input");
    write!(
        input,
        "\
CCO ethanol
not_a_smiles((( broken
c1ccccc1 benzene
"
    )
    .expect("write temp input");

    let output = yomitoki()
        .args(["analyze", "--input"])
        .arg(input.path())
        .args(["--format", "jsonl"])
        .output()
        .expect("run yomitoki");

    // At least one record failed, so the process must signal that via a
    // non-zero exit code -- but it must still have processed every record.
    assert!(!output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("utf8");
    let lines: Vec<&str> = stdout.lines().collect();
    assert_eq!(
        lines.len(),
        3,
        "expected one JSON line per record: {lines:?}"
    );
    assert!(lines[0].contains("\"input\":\"ethanol\""));
    assert!(lines[0].contains("\"report\":"));
    assert!(lines[1].contains("\"error\":"));
    assert!(lines[2].contains("\"input\":\"benzene\""));
    assert!(lines[2].contains("\"report\":"));
}

#[test]
fn sdf_batch_mode_continues_past_a_bad_record() {
    // Mirrors the .smi test above but for the .sdf path, which reads via
    // `chematic::mol::SdfReader` instead of `SmilesRecordReader` -- a
    // separate iterator implementation with its own error semantics that
    // needs its own proof it doesn't truncate on a bad record.
    let good_a = "\
mol_a
  chematic

  2  1  0  0  0  0  0  0  0  0  0 V2000
    0.0000    0.0000    0.0000 C   0  0  0  0  0  0  0  0  0  0  0  0
    1.0000    0.0000    0.0000 C   0  0  0  0  0  0  0  0  0  0  0  0
  1  2  1  0
M  END
$$$$
";
    // Malformed counts line ("  X  Y" instead of numeric atom/bond counts) --
    // the same shape chematic's own `test_sdf_reader_stops_on_error` uses to
    // provoke a `MolParseError`.
    let bad = "\
bad
  prog

  X  Y
M  END
$$$$
";
    let good_b = "\
mol_b
  chematic

  3  2  0  0  0  0  0  0  0  0  0 V2000
    0.0000    0.0000    0.0000 C   0  0  0  0  0  0  0  0  0  0  0  0
    1.0000    0.0000    0.0000 N   0  0  0  0  0  0  0  0  0  0  0  0
    2.0000    0.0000    0.0000 O   0  0  0  0  0  0  0  0  0  0  0  0
  1  2  1  0
  2  3  2  0
M  END
$$$$
";

    let mut input = tempfile::Builder::new()
        .suffix(".sdf")
        .tempfile()
        .expect("create temp input");
    write!(input, "{good_a}{bad}{good_b}").expect("write temp input");

    let output = yomitoki()
        .args(["analyze", "--input"])
        .arg(input.path())
        .args(["--format", "jsonl"])
        .output()
        .expect("run yomitoki");

    assert!(!output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("utf8");
    let lines: Vec<&str> = stdout.lines().collect();
    assert_eq!(
        lines.len(),
        3,
        "expected one JSON line per record, including the bad one: {lines:?}"
    );
    assert!(lines[0].contains("\"input\":\"mol_a\""));
    assert!(lines[0].contains("\"report\":"));
    assert!(lines[1].contains("\"error\":"));
    assert!(lines[2].contains("\"input\":\"mol_b\""));
    assert!(lines[2].contains("\"report\":"));
}

#[test]
fn output_flag_writes_to_a_file_instead_of_stdout() {
    let mut input = NamedTempFile::new().expect("create temp input");
    writeln!(input, "CCO").expect("write temp input");
    let output_file = NamedTempFile::new().expect("create temp output");

    let status = yomitoki()
        .args(["analyze", "--input"])
        .arg(input.path())
        .args(["--output"])
        .arg(output_file.path())
        .status()
        .expect("run yomitoki");
    assert!(status.success());

    let written = std::fs::read_to_string(output_file.path()).expect("read output file");
    assert!(written.contains("Verdict:"));
}
