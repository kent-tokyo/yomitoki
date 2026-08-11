#!/usr/bin/env python3
"""FM-1 (ring-family multiplicity) validation snapshot for the production
`ring_topology` L2-norm aggregation (adopted round 22 part 11 -- see
`../DEVELOPMENT_SET.md` Part 7 for the development-set evidence that led
to adoption).

Runs against the standard `mpscore_yomitoki.jsonl` (the real yomitoki
binary's output, produced by `run_yomitoki.py`) -- no dependency on any
diagnostic-only Rust API; the "complex ring stratum" (>=2 rings) is
defined via RDKit's own ring count (`descriptors()`, already used
throughout this harness), same as `evaluate_mpscore.py`'s other
stratifications.

This reports the CURRENT production binary's metrics as a standalone
snapshot, not a before/after diff -- the L1-vs-L2 comparison that
justified adoption is a one-time, already-decided, already-documented
result (DEVELOPMENT_SET.md Part 7), not something to keep re-deriving.
For a future before/after comparison (e.g. an FM-2 change), reuse
`metrics.paired_bootstrap_diff_ci` directly.

TS1/TS2/TS3 are NOT touched by this script -- MPScore development set
only.
"""

import json
import sys
import warnings
from pathlib import Path

warnings.filterwarnings("ignore")
import numpy as np
from sklearn.metrics import roc_auc_score

sys.path.insert(0, str(Path(__file__).resolve().parent))
from evaluate_mpscore import RESULTS_DIR, THRESHOLD, descriptors, load_labels, load_smiles, load_yomitoki


def build_rows(labels, smiles, yomitoki):
    rows = []
    for mol_id, row in labels.items():
        if row["hard_consensus"] == "":
            continue
        y_rec = yomitoki.get(mol_id)
        smi = smiles.get(mol_id)
        if y_rec is None or smi is None:
            continue
        d = descriptors(smi)
        if d is None:
            continue
        rows.append(
            {
                "label": int(row["hard_consensus"]),
                "rings": d["rings"],
                "overall_difficulty": y_rec["yomitoki_difficulty"],
                "ring_contribution": y_rec["yomitoki_components"]["ring_topology"]["contribution"],
            }
        )
    return rows


def rates(y_true, y_score, threshold=THRESHOLD):
    y_true = np.asarray(y_true)
    pred_hard = np.asarray(y_score) >= threshold
    n_easy = int(np.sum(y_true == 0))
    n_hard = int(np.sum(y_true == 1))
    fp = int(np.sum((y_true == 0) & pred_hard))
    fn = int(np.sum((y_true == 1) & ~pred_hard))
    return {
        "fp_rate": fp / n_easy if n_easy else None,
        "fn_rate": fn / n_hard if n_hard else None,
        "fp": fp,
        "fn": fn,
        "n_easy": n_easy,
        "n_hard": n_hard,
    }


def auc_or_none(y_true, y_score):
    if len(set(np.asarray(y_true).tolist())) < 2:
        return None
    return float(roc_auc_score(y_true, y_score))


def main():
    labels = load_labels()
    smiles = load_smiles()
    yomitoki = load_yomitoki()
    rows = build_rows(labels, smiles, yomitoki)

    y_true = np.array([r["label"] for r in rows])
    overall = np.array([r["overall_difficulty"] for r in rows])
    ring = np.array([r["ring_contribution"] for r in rows])
    complex_idx = np.array([i for i, r in enumerate(rows) if r["rings"] >= 2])
    y_true_complex = y_true[complex_idx]

    report = {
        "n_molecules_considered": len(rows),
        "n_complex_stratum": len(complex_idx),
        "full_set_overall_auc": auc_or_none(y_true, overall),
        "complex_stratum_overall_auc": auc_or_none(y_true_complex, overall[complex_idx]),
        "complex_stratum_ring_only_auc": auc_or_none(y_true_complex, ring[complex_idx]),
        "full_set_rates": rates(y_true, overall),
        "complex_stratum_rates": rates(y_true_complex, overall[complex_idx]),
        "complex_stratum_mean_ring_contribution": {
            "hard": float(np.mean(ring[complex_idx][y_true_complex == 1])),
            "easy": float(np.mean(ring[complex_idx][y_true_complex == 0])),
        },
    }

    out_path = RESULTS_DIR / "ring_family_l2_full_evaluation.json"
    with open(out_path, "w") as f:
        json.dump(report, f, indent=2)
    print(f"wrote {out_path}", file=sys.stderr)

    print(f"\nn_molecules={report['n_molecules_considered']} n_complex_stratum={report['n_complex_stratum']}\n")
    print(f"full_set_overall_auc            = {report['full_set_overall_auc']:.4f}")
    print(f"complex_stratum_overall_auc      = {report['complex_stratum_overall_auc']:.4f}")
    print(f"complex_stratum_ring_only_auc    = {report['complex_stratum_ring_only_auc']:.4f}")

    fr = report["full_set_rates"]
    cr = report["complex_stratum_rates"]
    mc = report["complex_stratum_mean_ring_contribution"]
    print(f"\nfull FP_rate={fr['fp_rate']:.4f} FN_rate={fr['fn_rate']:.4f}")
    print(f"complex FP_rate={cr['fp_rate']:.4f} FN_rate={cr['fn_rate']:.4f}")
    print(f"complex mean_ring_contribution hard={mc['hard']:.4f} easy={mc['easy']:.4f}")


if __name__ == "__main__":
    main()
