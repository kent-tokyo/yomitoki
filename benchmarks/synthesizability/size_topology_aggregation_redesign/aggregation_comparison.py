#!/usr/bin/env python3
"""Sections 5-7: parameter-free aggregation candidate comparison.

Only the three burden terms' AGGREGATION changes -- MW/RB/heteroatom
weights (m, r, h themselves) and the outer saturation transform
(`1 - exp(-aggregated/2)`, SIZE_BURDEN_SCALE fixed) are untouched.

Candidates (parameter-free, as instructed -- no Lp grid search, no
mixture coefficient, no learned weights):
- L1_current: m + r + h (production today)
- L2: sqrt(m^2 + r^2 + h^2) -- same functional form this project already
  adopted for ring_topology's ring-family aggregation (round 22 part 11)
- MAX: max(m, r, h)

A 4th candidate (LogSumExp, considered) was deliberately NOT added: it
fails to preserve the zero-term identity property L1/L2/MAX all share
(LogSumExp(x,0,0) = ln(exp(x)+2) != x), which would inflate EVERY
molecule's burden even when two of the three terms are exactly zero
(e.g. any zero-rotatable-bond, zero-heteroatom molecule) -- a clear
regression against `small_molecule_has_low_size_burden`-type invariants
this project already tests for. Not worth the complexity given the
round explicitly made a 4th candidate optional.

No fitting happens here (all three forms are closed-form, zero free
parameters) -- so no train/val split is needed for validity, unlike the
probes in discriminator_probe.py. Scaffold folds are used only as a
population-consistency check (does the candidate help uniformly across
folds, or is it driven by one subpopulation), reported per-fold
alongside the full-set numbers.
"""

import json
import sys
import warnings

import numpy as np
from sklearn.metrics import average_precision_score

warnings.filterwarnings("ignore")

sys.path.insert(0, "../scripts")
from metrics import paired_bootstrap_diff_ci  # noqa: E402

from common import AGGREGATORS, N_FOLDS, RESULTS_DIR, classification_report, load_dataset, overall_difficulty, saturate  # noqa: E402


def score_for_aggregator(df, agg_fn):
    raw = agg_fn(df["mw_burden"].to_numpy(), df["rotatable_burden"].to_numpy(), df["heteroatom_burden"].to_numpy())
    size_c = saturate(raw)
    return size_c, overall_difficulty(df["ring_contribution"].to_numpy(), size_c, df["stereo_contribution"].to_numpy(), df["fg_contribution"].to_numpy())


def main():
    df = load_dataset()
    y = df["expert_label"].to_numpy()

    scores = {}
    for name, agg_fn in AGGREGATORS.items():
        size_c, overall = score_for_aggregator(df, agg_fn)
        scores[name] = overall

    report = {"full_set": {}, "per_fold": {}, "paired_bootstrap_vs_L1": {}}
    for name, overall in scores.items():
        report["full_set"][name] = classification_report(y, overall)

    for name, overall in scores.items():
        per_fold = []
        for fold_idx in range(N_FOLDS):
            mask = (df["fold"] == fold_idx).to_numpy()
            per_fold.append({"fold": fold_idx, **classification_report(y[mask], overall[mask])})
        report["per_fold"][name] = per_fold

    for name in ["L2", "MAX"]:
        ci_auc = paired_bootstrap_diff_ci(y, scores["L1_current"], scores[name])
        ci_pr = paired_bootstrap_diff_ci(y, scores["L1_current"], scores[name], metric_fn=lambda yt, ys: average_precision_score(yt, ys))
        report["paired_bootstrap_vs_L1"][name] = {"roc_auc_diff": ci_auc, "hard_pr_auc_diff": ci_pr}

    RESULTS_DIR.mkdir(parents=True, exist_ok=True)
    with open(RESULTS_DIR / "aggregation_comparison.json", "w") as f:
        json.dump(report, f, indent=2)

    print("=== Full-set metrics ===")
    for name in AGGREGATORS:
        m = report["full_set"][name]
        print(f"{name:12s}  ROC-AUC={m['roc_auc']:.4f}  hardPR-AUC={m['hard_class_pr_auc']:.4f}  BalAcc={m['balanced_accuracy']:.4f}  MCC={m['mcc']:.4f}  FP={m['fp']}  FN={m['fn']}")

    print("\n=== Paired bootstrap vs L1_current ===")
    for name, d in report["paired_bootstrap_vs_L1"].items():
        auc = d["roc_auc_diff"]
        print(f"{name}: ROC-AUC diff point={auc['point']:.4f}  95% CI [{auc['low']:.4f}, {auc['high']:.4f}]")

    print("\n=== Per-fold ROC-AUC (consistency check) ===")
    for name in AGGREGATORS:
        vals = [f["roc_auc"] for f in report["per_fold"][name]]
        print(f"{name:12s}  {[round(v,4) for v in vals]}")


if __name__ == "__main__":
    main()
