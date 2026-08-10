#!/usr/bin/env python3
"""Joins ground truth + per-method raw scores for TS1/TS2/TS3, normalizes
score direction, computes accuracy metrics (+ bootstrap CI) for every
method on the exact same molecule set, and computes yomitoki's selective
prediction (risk-coverage/AURC) and confidence calibration.

Binarization thresholds (only used for accuracy/precision/recall/F1/MCC/
confusion matrix -- ROC-AUC/PR-AUC are threshold-free and are the primary
metrics per this benchmark's own methodology, see ../README.md):

  - yomitoki: 0.5, the score's own DIFFICULTY_MODERATE_MAX boundary
    (src/rules.rs) -- the pre-existing verdict boundary between
    "still on the accessible side" and "challenging side", not fit to
    this test set.
  - sascore / brsascore: 5.5, the un-tuned midpoint of the methods' own
    stated 1-10 range. Neither method's own paper publishes a
    recommended binary decision threshold (both report only threshold-free
    AUC numbers) -- the midpoint is used as a neutral, non-data-derived
    convention, not a value chosen by looking at these results.

This script never adjusts a threshold based on what it improves on this
data -- that would be exactly the test-set-fitting this benchmark's
methodology forbids.

Coverage handling: a molecule missing from a method's output, or present
with `error != null`, is excluded from THAT method's metrics and counted
in its `coverage` stats -- never silently dropped without being counted.
"""

import argparse
import csv
import json
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent))
from metrics import aurc, bootstrap_ci, calibration_bins, classification_metrics, expected_calibration_error, risk_coverage_curve
from normalize_direction import normalize
from sklearn.metrics import average_precision_score, roc_auc_score

THRESHOLDS = {"yomitoki": 0.5, "sascore": 5.5, "brsascore": 5.5}
RESULTS_DIR = Path(__file__).parent.parent / "results"
DATASETS_DIR = Path(__file__).parent.parent / "datasets" / "downloaded" / "brsascore"


def load_labels(dataset: str) -> dict:
    path = DATASETS_DIR / f"{dataset}_labels.csv"
    with open(path) as f:
        return {row["id"]: int(row["label"]) for row in csv.DictReader(f)}


def load_yomitoki(dataset: str) -> dict:
    path = RESULTS_DIR / "raw" / f"{dataset}_yomitoki.jsonl"
    out = {}
    with open(path) as f:
        for line in f:
            rec = json.loads(line)
            if rec["error"] is not None:
                continue
            out[rec["id"]] = rec
    return out


def load_scores(dataset: str, method: str, score_field: str) -> dict:
    path = RESULTS_DIR / "raw" / f"{dataset}_{method}.jsonl"
    out = {}
    with open(path) as f:
        for line in f:
            rec = json.loads(line)
            if rec["error"] is not None or rec[score_field] is None:
                continue
            out[rec["id"]] = rec[score_field]
    return out


def evaluate_method(dataset: str, method: str, labels: dict, raw_scores: dict) -> dict:
    total = len(labels)
    ids = [i for i in labels if i in raw_scores]
    failed = total - len(ids)
    y_true = [labels[i] for i in ids]
    y_score = [normalize(method, raw_scores[i]) for i in ids]

    coverage = {"total": total, "scored": len(ids), "failed": failed, "coverage_fraction": len(ids) / total if total else 0.0}

    if len(set(y_true)) < 2 or not ids:
        return {"dataset": dataset, "method": method, "coverage": coverage, "metrics": None, "note": "insufficient class diversity or no scored molecules"}

    threshold = THRESHOLDS[method]
    cm = classification_metrics(y_true, y_score, threshold=threshold)
    roc_ci = bootstrap_ci(y_true, y_score, metric_fn=lambda yt, ys: roc_auc_score(yt, ys))
    pr_ci = bootstrap_ci(y_true, y_score, metric_fn=lambda yt, ys: average_precision_score(yt, ys))

    return {
        "dataset": dataset,
        "method": method,
        "coverage": coverage,
        "threshold": threshold,
        "metrics": cm.to_dict(),
        "roc_auc_95ci": roc_ci,
        "pr_auc_95ci": pr_ci,
    }


def evaluate_yomitoki_selective(dataset: str, labels: dict, yomitoki: dict) -> dict:
    ids = [i for i in labels if i in yomitoki]
    y_true = [labels[i] for i in ids]
    difficulty = [yomitoki[i]["yomitoki_difficulty"] for i in ids]
    confidence = [yomitoki[i]["yomitoki_confidence"] for i in ids]
    y_pred = [1 if d >= THRESHOLDS["yomitoki"] else 0 for d in difficulty]
    y_correct = [1 if p == t else 0 for p, t in zip(y_pred, y_true)]

    rc_points = risk_coverage_curve(y_correct, confidence)
    bins = calibration_bins(y_correct, confidence)
    ece = expected_calibration_error(bins, len(ids))

    return {
        "dataset": dataset,
        "n": len(ids),
        "aurc": aurc(y_correct, confidence),
        "risk_coverage": [vars(p) for p in rc_points],
        "calibration_bins": [vars(b) for b in bins],
        "expected_calibration_error": ece,
        "brier_score_note": "yomitoki confidence is NOT claimed to be a correctness-probability estimate -- ECE/calibration bins reported as a ranking-usefulness check only, see docs/benchmark.md",
    }


def write_per_molecule(dataset: str, labels: dict, yomitoki: dict, sascore: dict, brsascore: dict, out_f):
    for mol_id, label in labels.items():
        y = yomitoki.get(mol_id)
        rec = {
            "id": mol_id,
            "dataset": dataset,
            "ground_truth": label,
            "yomitoki_difficulty": y["yomitoki_difficulty"] if y else None,
            "yomitoki_confidence": y["yomitoki_confidence"] if y else None,
            "yomitoki_verdict": y["yomitoki_verdict"] if y else None,
            "yomitoki_prediction": (1 if y["yomitoki_difficulty"] >= THRESHOLDS["yomitoki"] else 0) if y else None,
            "sascore": sascore.get(mol_id),
            "brsascore": brsascore.get(mol_id),
        }
        if y is not None:
            rec["yomitoki_correct"] = int(rec["yomitoki_prediction"] == label)
        out_f.write(json.dumps(rec) + "\n")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--datasets", nargs="+", default=["ts1", "ts2", "ts3"])
    args = parser.parse_args()

    accuracy_results = []
    selective_results = []
    per_molecule_path = RESULTS_DIR / "per_molecule.jsonl"
    per_molecule_path.parent.mkdir(parents=True, exist_ok=True)

    with open(per_molecule_path, "w") as pm_f:
        for dataset in args.datasets:
            labels = load_labels(dataset)
            yomitoki = load_yomitoki(dataset)
            sascore = load_scores(dataset, "sascore", "sascore")
            brsascore = load_scores(dataset, "brsascore", "brsascore")

            accuracy_results.append(evaluate_method(dataset, "yomitoki", labels, {i: r["yomitoki_difficulty"] for i, r in yomitoki.items()}))
            accuracy_results.append(evaluate_method(dataset, "sascore", labels, sascore))
            accuracy_results.append(evaluate_method(dataset, "brsascore", labels, brsascore))
            selective_results.append(evaluate_yomitoki_selective(dataset, labels, yomitoki))

            write_per_molecule(dataset, labels, yomitoki, sascore, brsascore, pm_f)
            print(f"{dataset}: {len(labels)} labeled molecules processed", file=sys.stderr)

    summary_path = RESULTS_DIR / "benchmark_summary.json"
    with open(summary_path, "w") as f:
        json.dump({"accuracy": accuracy_results, "selective_prediction": selective_results}, f, indent=2)

    selective_csv_path = RESULTS_DIR / "selective_prediction.csv"
    with open(selective_csv_path, "w", newline="") as f:
        writer = csv.writer(f)
        writer.writerow(["dataset", "coverage", "threshold", "n_covered", "risk", "precision", "recall"])
        for sel in selective_results:
            for p in sel["risk_coverage"]:
                writer.writerow([sel["dataset"], p["coverage"], p["threshold"], p["n_covered"], p["risk"], p["precision"], p["recall"]])

    print(f"wrote {summary_path}, {selective_csv_path}, {per_molecule_path}", file=sys.stderr)


if __name__ == "__main__":
    main()
