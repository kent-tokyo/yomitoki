#!/usr/bin/env python3
"""Phase C: throughput benchmark for yomitoki/SAscore/BR-SAScore on the
same machine, same TS1/TS2/TS3 inputs.

Methodology, stated explicitly because the three methods are NOT measured
identically (disclosed, not hidden -- see docs/benchmark.md's
Limitations section):

  - yomitoki: measured via subprocess `yomitoki analyze --format jsonl`,
    i.e. process startup + SMILES parsing + full analysis, all combined
    (this project's CLI has no separate "parse-only" mode to subtract
    out; a Rust-level micro-benchmark isolating analyze-only time is
    listed as future work, not attempted this round).
  - SAscore / BR-SAScore: measured in-process (no subprocess overhead),
    with RDKit parsing (`Chem.MolFromSmiles`) done BEFORE the timed
    region -- i.e. analyze-only, parsing excluded.

Because of this asymmetry, cross-method throughput numbers here should
be read as "same order of magnitude or not," not as a precise ratio.
First call to each method is a discarded warm-up (JIT/cache effects,
Python import cost) per method, excluded from reported numbers.
"""

import json
import subprocess
import sys
import time
import warnings
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[3]
YOMITOKI_BIN = REPO_ROOT / "target" / "release" / "yomitoki"
DATASETS_DIR = Path(__file__).parent.parent / "datasets" / "downloaded" / "brsascore"
RESULTS_DIR = Path(__file__).parent.parent / "results"


def time_yomitoki(path: Path) -> tuple[float, int]:
    t0 = time.perf_counter()
    r = subprocess.run([str(YOMITOKI_BIN), "analyze", "--input", str(path), "--format", "jsonl"], capture_output=True, text=True)
    t1 = time.perf_counter()
    n = len([line for line in r.stdout.splitlines() if line.strip()])
    return t1 - t0, n


def time_sascore(path: Path) -> tuple[float, int]:
    import rdkit

    sys.path.insert(0, str(Path(rdkit.__file__).parent / "Contrib" / "SA_Score"))
    import sascorer
    from rdkit import Chem

    mols = [Chem.MolFromSmiles(line.split("\t")[0]) for line in open(path) if line.strip()]
    t0 = time.perf_counter()
    for m in mols:
        if m is not None:
            sascorer.calculateScore(m)
    t1 = time.perf_counter()
    return t1 - t0, len(mols)


def time_brsascore(path: Path) -> tuple[float, int]:
    from BRSAScore.BRSAScore import SAScorer

    scorer = SAScorer()
    smiles = [line.split("\t")[0] for line in open(path) if line.strip()]
    t0 = time.perf_counter()
    for smi in smiles:
        try:
            scorer.calculateScore(smi)
        except Exception:
            pass
    t1 = time.perf_counter()
    return t1 - t0, len(smiles)


def machine_info() -> dict:
    import platform

    return {
        "platform": platform.platform(),
        "processor": platform.processor() or platform.machine(),
        "python_version": platform.python_version(),
        "rustc_version": subprocess.run(["rustc", "--version"], capture_output=True, text=True).stdout.strip(),
        "build": "release",
    }


def main():
    warnings.filterwarnings("ignore")
    from rdkit import RDLogger

    RDLogger.DisableLog("rdApp.*")

    if not YOMITOKI_BIN.exists():
        raise SystemExit(f"yomitoki binary not found at {YOMITOKI_BIN} -- build with: cargo build --release --bin yomitoki")

    datasets = ["ts1", "ts2", "ts3"]
    timers = {"yomitoki": time_yomitoki, "sascore": time_sascore, "brsascore": time_brsascore}

    # warm-up: one discarded call per method, excluded from reported numbers
    for name, fn in timers.items():
        fn(DATASETS_DIR / "ts1.smi")

    results = {"machine": machine_info(), "methods": {}}
    for name, fn in timers.items():
        results["methods"][name] = {}
        for ds in datasets:
            elapsed, n = fn(DATASETS_DIR / f"{ds}.smi")
            results["methods"][name][ds] = {
                "elapsed_s": elapsed,
                "n_molecules": n,
                "ms_per_molecule": elapsed * 1000 / n,
                "molecules_per_sec": n / elapsed,
            }
            print(f"{name:10s} {ds}: n={n} elapsed={elapsed:.3f}s ms/mol={elapsed * 1000 / n:.3f} mol/s={n / elapsed:.1f}", file=sys.stderr)

    RESULTS_DIR.mkdir(parents=True, exist_ok=True)
    out_path = RESULTS_DIR / "timing.json"
    with open(out_path, "w") as f:
        json.dump(results, f, indent=2)
    print(f"wrote {out_path}", file=sys.stderr)


if __name__ == "__main__":
    main()
