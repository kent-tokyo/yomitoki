#!/usr/bin/env python3
"""Runs the real `yomitoki` CLI (release build, `AnalysisConfig::default()`
-- the frozen GeneralOrganic default, no fragment corpus configured unless
`--fragment-corpus` is explicitly passed) over an input molecule set and
writes one JSON record per molecule.

Batch mode is used deliberately (one process, corpus loaded once if
configured) rather than one subprocess per molecule -- see
tasks/upstream_and_corpus_research.md's round-20 note on why looping the
CLI per-molecule is wasteful.

Input format: a whitespace/tab-delimited file, one molecule per line:
`<smiles> <id>` (matches `chematic::mol::SmilesRecordReader`'s default,
the same shape `tools/build-fragment-corpus` and the round-19/20/21
validation scripts already use).

Output: JSONL, one record per input line, id-keyed so downstream scripts
can join against ground truth and other methods' scores without assuming
input order is preserved end-to-end.
"""

import argparse
import json
import subprocess
import sys
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[3]
DEFAULT_BIN = REPO_ROOT / "target" / "release" / "yomitoki"


def run(input_path: Path, bin_path: Path, fragment_corpus: Path | None) -> list[dict]:
    if not bin_path.exists():
        raise SystemExit(
            f"yomitoki binary not found at {bin_path} -- build it first: "
            f"cargo build --release --bin yomitoki"
        )
    args = [str(bin_path), "analyze", "--input", str(input_path), "--format", "jsonl"]
    if fragment_corpus is not None:
        args += ["--fragment-corpus", str(fragment_corpus)]

    result = subprocess.run(args, capture_output=True, text=True)
    # Batch mode's own contract (AGENTS.md §15): exit code is non-zero if
    # *any* record failed, but every record is still attempted and
    # written -- never treat a non-zero exit here as "no output to parse".
    lines = [line for line in result.stdout.splitlines() if line.strip()]
    if not lines:
        raise SystemExit(
            f"yomitoki produced no output -- stderr:\n{result.stderr}"
        )

    records = []
    for line in lines:
        rec = json.loads(line)
        mol_id = rec["input"]
        if "error" in rec:
            records.append({"id": mol_id, "error": rec["error"], "report": None})
            continue
        report = rec["report"]
        records.append(
            {
                "id": mol_id,
                "error": None,
                "yomitoki_difficulty": report["overall"]["difficulty"],
                "yomitoki_synthesizability": report["overall"]["synthesizability"],
                "yomitoki_confidence": report["overall"]["confidence"],
                "yomitoki_verdict": report["overall"]["verdict"],
                "yomitoki_applicability": report["applicability"],
                "yomitoki_fragment_precedent": report.get("fragment_precedent"),
                "yomitoki_findings": [f["code"] for f in report["findings"]],
                "yomitoki_components": report["components"],
            }
        )
    return records


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("input", type=Path, help="whitespace-delimited <smiles> <id> file")
    parser.add_argument("output", type=Path, help="output JSONL path")
    parser.add_argument("--bin", type=Path, default=DEFAULT_BIN, help="path to the yomitoki binary")
    parser.add_argument(
        "--fragment-corpus",
        type=Path,
        default=None,
        help="optional: exercise fragment_precedent evidence too (never affects "
        "overall.difficulty as of v0.1.0 -- see rules.rs's 'Fragment precedent' section)",
    )
    args = parser.parse_args()

    records = run(args.input, args.bin, args.fragment_corpus)
    args.output.parent.mkdir(parents=True, exist_ok=True)
    with open(args.output, "w") as f:
        for rec in records:
            f.write(json.dumps(rec) + "\n")

    n_ok = sum(1 for r in records if r["error"] is None)
    n_err = len(records) - n_ok
    print(f"wrote {len(records)} records ({n_ok} ok, {n_err} parse errors) to {args.output}", file=sys.stderr)


if __name__ == "__main__":
    main()
