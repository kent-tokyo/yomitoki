#!/usr/bin/env python3
"""Information-Loss Audit, step 3 (round 22 part 13): diagnostic probes.

NOT a production-ML round -- the question is "does information exist in
this feature set," not "what's the best model." Two probes per feature
set:
  Model A: L2-regularized logistic regression (standardized), a linear
    probe -- can a straight line separate the classes in this space?
  Model B: shallow HistGradientBoostingClassifier (max_depth=3,
    max_iter=100 -- fixed, not tuned) -- can *some* nonlinear/interaction
    structure separate them, that a linear probe would miss?

Both use class-balanced sample weighting (MPScore is ~87% hard) so
balanced-accuracy/MCC aren't dominated by the trivial always-hard
prediction; ROC-AUC/PR-AUC are threshold-free and unaffected either way.

Scaffold-grouped 5-fold CV (scaffold_cv.py's fold assignment) throughout
-- never a random split.
"""

import json
import sys
import warnings

import numpy as np
import pandas as pd
from scipy import stats
from sklearn.ensemble import HistGradientBoostingClassifier
from sklearn.linear_model import LogisticRegression
from sklearn.metrics import average_precision_score, balanced_accuracy_score, matthews_corrcoef, roc_auc_score
from sklearn.preprocessing import StandardScaler
from sklearn.utils.class_weight import compute_sample_weight

warnings.filterwarnings("ignore")

from common import FEATURE_SETS, N_FOLDS, RESULTS_DIR, SEED

THRESHOLD = 0.5


def fit_predict_linear(X_train, y_train, X_val):
    scaler = StandardScaler()
    X_train_s = scaler.fit_transform(X_train)
    X_val_s = scaler.transform(X_val)
    sw = compute_sample_weight("balanced", y_train)
    clf = LogisticRegression(penalty="l2", C=1.0, max_iter=2000, random_state=SEED)
    clf.fit(X_train_s, y_train, sample_weight=sw)
    return clf.predict_proba(X_val_s)[:, 1]


def fit_predict_nonlinear(X_train, y_train, X_val):
    sw = compute_sample_weight("balanced", y_train)
    clf = HistGradientBoostingClassifier(max_depth=3, max_iter=100, random_state=SEED)
    clf.fit(X_train, y_train, sample_weight=sw)
    return clf.predict_proba(X_val)[:, 1]


def compute_metrics(y_true, y_score):
    y_true = np.asarray(y_true)
    y_score = np.asarray(y_score)
    y_pred = (y_score >= THRESHOLD).astype(int)
    return {
        "roc_auc": float(roc_auc_score(y_true, y_score)) if len(set(y_true)) > 1 else None,
        "balanced_accuracy": float(balanced_accuracy_score(y_true, y_pred)),
        "mcc": float(matthews_corrcoef(y_true, y_pred)),
        "hard_class_pr_auc": float(average_precision_score(y_true, y_score)) if len(set(y_true)) > 1 else None,
        "easy_class_pr_auc": float(average_precision_score(1 - y_true, 1 - y_score)) if len(set(y_true)) > 1 else None,
    }


def run_feature_set(df, columns, probe_fn):
    fold_results = []
    for fold_idx in range(N_FOLDS):
        train = df[df["fold"] != fold_idx]
        val = df[df["fold"] == fold_idx]
        X_train, y_train = train[columns].to_numpy(), train["expert_label"].to_numpy()
        X_val, y_val = val[columns].to_numpy(), val["expert_label"].to_numpy()
        y_score = probe_fn(X_train, y_train, X_val)
        m = compute_metrics(y_val, y_score)
        m["fold"] = fold_idx
        m["n_val"] = int(len(val))
        fold_results.append(m)
    return fold_results


def summarize_folds(fold_results, metric):
    values = [r[metric] for r in fold_results if r[metric] is not None]
    if not values:
        return {"mean": None, "std": None, "values": []}
    return {"mean": float(np.mean(values)), "std": float(np.std(values)), "values": values}


def paired_fold_comparison(results_a, results_b, metric):
    """Fold-wise paired difference (b - a) across the SAME 5 folds --
    honest with n=5: reports the raw paired differences and a paired
    t-test p-value as a lightweight signal, not a strong claim. Low
    power with only 5 folds is stated explicitly in the report, not
    hidden.
    """
    a = [r[metric] for r in results_a if r[metric] is not None]
    b = [r[metric] for r in results_b if r[metric] is not None]
    if len(a) != len(b) or len(a) < 2:
        return None
    diffs = [bi - ai for ai, bi in zip(a, b)]
    t_stat, p_value = stats.ttest_rel(b, a)
    return {
        "mean_diff": float(np.mean(diffs)),
        "std_diff": float(np.std(diffs)),
        "diffs_by_fold": diffs,
        "paired_ttest_p_value": float(p_value),
        "n_folds": len(a),
        "caveat": "n=5 folds only -- low statistical power, read as a directional signal not a significance proof",
    }


def main():
    df = pd.read_csv(RESULTS_DIR / "features_with_folds.csv")

    all_results = {}
    for name, columns in FEATURE_SETS.items():
        print(f"running {name} ({len(columns)} features)...", file=sys.stderr)
        linear = run_feature_set(df, columns, fit_predict_linear)
        nonlinear = run_feature_set(df, columns, fit_predict_nonlinear)
        all_results[name] = {"linear": linear, "nonlinear": nonlinear, "columns": columns}

    # Headline comparisons: F1 (raw descriptors) vs F0 (current components),
    # both probes, on ROC-AUC specifically -- the round's central question.
    comparisons = {}
    for probe in ["linear", "nonlinear"]:
        for metric in ["roc_auc", "balanced_accuracy", "mcc", "hard_class_pr_auc", "easy_class_pr_auc"]:
            comparisons[f"F1_vs_F0_{probe}_{metric}"] = paired_fold_comparison(
                all_results["F0_components"][probe], all_results["F1_raw_descriptors"][probe], metric
            )
    # nonlinear vs linear, within each feature set -- Case C's evidence.
    for name in FEATURE_SETS:
        comparisons[f"{name}_nonlinear_vs_linear_roc_auc"] = paired_fold_comparison(
            all_results[name]["linear"], all_results[name]["nonlinear"], "roc_auc"
        )

    summary = {"feature_sets": {}, "comparisons": comparisons}
    for name, res in all_results.items():
        summary["feature_sets"][name] = {
            "columns": res["columns"],
            "linear": {m: summarize_folds(res["linear"], m) for m in ["roc_auc", "balanced_accuracy", "mcc", "hard_class_pr_auc", "easy_class_pr_auc"]},
            "nonlinear": {m: summarize_folds(res["nonlinear"], m) for m in ["roc_auc", "balanced_accuracy", "mcc", "hard_class_pr_auc", "easy_class_pr_auc"]},
            "linear_fold_detail": res["linear"],
            "nonlinear_fold_detail": res["nonlinear"],
        }

    with open(RESULTS_DIR / "probe_results.json", "w") as f:
        json.dump(summary, f, indent=2)

    print(f"\n{'feature_set':<20} {'linear AUC':>12} {'nonlinear AUC':>14} {'bal_acc(lin)':>13} {'MCC(lin)':>10}", file=sys.stderr)
    for name, res in summary["feature_sets"].items():
        lin_auc = res["linear"]["roc_auc"]["mean"]
        nl_auc = res["nonlinear"]["roc_auc"]["mean"]
        bal = res["linear"]["balanced_accuracy"]["mean"]
        mcc = res["linear"]["mcc"]["mean"]
        print(f"{name:<20} {lin_auc:>12.4f} {nl_auc:>14.4f} {bal:>13.4f} {mcc:>10.4f}", file=sys.stderr)

    print("\nF1 vs F0 (raw descriptors vs current components), ROC-AUC:", file=sys.stderr)
    for probe in ["linear", "nonlinear"]:
        c = comparisons[f"F1_vs_F0_{probe}_roc_auc"]
        print(f"  {probe}: mean_diff={c['mean_diff']:.4f} p={c['paired_ttest_p_value']:.4f} diffs={[round(d,4) for d in c['diffs_by_fold']]}", file=sys.stderr)


if __name__ == "__main__":
    main()
