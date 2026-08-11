#!/usr/bin/env python3
"""Sections 2-3: transform-vs-aggregation discriminator.

A0: current F0 (4 components) alone.
A1: F0 + reconstructed raw total (mw_burden+rotatable_burden+
    heteroatom_burden, recovered by inverting the saturation transform
    on the real size_contribution) -- if the transform were destroying
    information, this should recover most of what A8 recovers, since it
    IS the raw total, just un-saturated. Per section 1, the transform is
    confirmed invertible, so A1 should add essentially nothing over A0.
A2-A7: F0 + each burden term / pair, isolating single- and
    two-primitive contributions.
A8: F0 + all three burden terms separately (not summed) -- if this beats
    A1 by a wide margin, the loss is in the L1-sum AGGREGATION, not the
    saturation transform (Case G).

Same probe methodology as round 14/18: L2-regularized logistic
regression primary, HistGradientBoostingClassifier secondary, 5-fold
scaffold CV, paired per-fold comparison.
"""

import json
import sys
import warnings

import numpy as np
from scipy import stats
from sklearn.ensemble import HistGradientBoostingClassifier
from sklearn.linear_model import LogisticRegression
from sklearn.metrics import average_precision_score, balanced_accuracy_score, matthews_corrcoef, roc_auc_score
from sklearn.preprocessing import StandardScaler
from sklearn.utils.class_weight import compute_sample_weight

warnings.filterwarnings("ignore")

from common import F0_COLUMNS, N_FOLDS, RESULTS_DIR, SEED, desaturate, load_dataset

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
    y_true, y_score = np.asarray(y_true), np.asarray(y_score)
    y_pred = (y_score >= THRESHOLD).astype(int)
    return {
        "roc_auc": float(roc_auc_score(y_true, y_score)) if len(set(y_true)) > 1 else None,
        "balanced_accuracy": float(balanced_accuracy_score(y_true, y_pred)),
        "mcc": float(matthews_corrcoef(y_true, y_pred)),
        "hard_class_pr_auc": float(average_precision_score(y_true, y_score)) if len(set(y_true)) > 1 else None,
    }


def run_feature_set(df, columns, probe_fn):
    fold_results = []
    for fold_idx in range(N_FOLDS):
        train, val = df[df["fold"] != fold_idx], df[df["fold"] == fold_idx]
        X_train, y_train = train[columns].to_numpy(), train["expert_label"].to_numpy()
        X_val, y_val = val[columns].to_numpy(), val["expert_label"].to_numpy()
        m = compute_metrics(y_val, probe_fn(X_train, y_train, X_val))
        m["fold"] = fold_idx
        fold_results.append(m)
    return fold_results


def summarize_folds(fold_results, metric):
    values = [r[metric] for r in fold_results if r[metric] is not None]
    return {"mean": float(np.mean(values)), "std": float(np.std(values)), "values": values} if values else {"mean": None, "std": None, "values": []}


def paired_fold_comparison(results_a, results_b, metric):
    a = [r[metric] for r in results_a if r[metric] is not None]
    b = [r[metric] for r in results_b if r[metric] is not None]
    diffs = [bi - ai for ai, bi in zip(a, b)]
    t_stat, p_value = stats.ttest_rel(b, a)
    return {
        "mean_diff": float(np.mean(diffs)),
        "all_folds_same_direction": all(d > 0 for d in diffs) or all(d < 0 for d in diffs),
        "paired_ttest_p_value": float(p_value),
        "diffs_by_fold": diffs,
    }


def main():
    df = load_dataset()
    df["raw_total_reconstructed"] = desaturate(df["size_contribution"])

    FEATURE_SETS = {
        "A0_F0_only": F0_COLUMNS,
        "A1_F0_plus_raw_total": F0_COLUMNS + ["raw_total_reconstructed"],
        "A2_F0_plus_mw": F0_COLUMNS + ["mw_burden"],
        "A3_F0_plus_rb": F0_COLUMNS + ["rotatable_burden"],
        "A4_F0_plus_het": F0_COLUMNS + ["heteroatom_burden"],
        "A5_F0_plus_mw_rb": F0_COLUMNS + ["mw_burden", "rotatable_burden"],
        "A6_F0_plus_mw_het": F0_COLUMNS + ["mw_burden", "heteroatom_burden"],
        "A7_F0_plus_rb_het": F0_COLUMNS + ["rotatable_burden", "heteroatom_burden"],
        "A8_F0_plus_all_decomposed": F0_COLUMNS + ["mw_burden", "rotatable_burden", "heteroatom_burden"],
    }

    all_results = {}
    for name, columns in FEATURE_SETS.items():
        print(f"running {name} ({columns})...", file=sys.stderr)
        all_results[name] = {
            "linear": run_feature_set(df, columns, fit_predict_linear),
            "nonlinear": run_feature_set(df, columns, fit_predict_nonlinear),
        }

    comparisons = {}
    for name in FEATURE_SETS:
        if name == "A0_F0_only":
            continue
        for probe in ["linear", "nonlinear"]:
            comparisons[f"{name}_vs_A0_{probe}"] = paired_fold_comparison(all_results["A0_F0_only"][probe], all_results[name][probe], "roc_auc")

    summary = {"feature_sets": {}, "comparisons": comparisons}
    for name, res in all_results.items():
        summary["feature_sets"][name] = {
            probe: {m: summarize_folds(res[probe], m) for m in ["roc_auc", "balanced_accuracy", "mcc", "hard_class_pr_auc"]}
            for probe in ["linear", "nonlinear"]
        }

    # Diagnosis (section 3): Case T if A1 >> A0; Case G if A1 ~= A0 but A8 >> A0.
    a1_delta = comparisons["A1_F0_plus_raw_total_vs_A0_linear"]["mean_diff"]
    a8_delta = comparisons["A8_F0_plus_all_decomposed_vs_A0_linear"]["mean_diff"]
    a1_delta_nl = comparisons["A1_F0_plus_raw_total_vs_A0_nonlinear"]["mean_diff"]
    a8_delta_nl = comparisons["A8_F0_plus_all_decomposed_vs_A0_nonlinear"]["mean_diff"]

    TRANSFORM_LOSS_THRESHOLD = 0.02  # A1 delta above this = meaningful transform-side recovery
    AGGREGATION_LOSS_THRESHOLD = 0.02  # A8 delta above this, with A1 below = aggregation-side loss

    transform_loss = a1_delta >= TRANSFORM_LOSS_THRESHOLD and a1_delta_nl >= TRANSFORM_LOSS_THRESHOLD
    aggregation_loss = a8_delta >= AGGREGATION_LOSS_THRESHOLD and a8_delta_nl >= AGGREGATION_LOSS_THRESHOLD and a1_delta < TRANSFORM_LOSS_THRESHOLD

    if transform_loss and aggregation_loss:
        diagnosis = "M (mixed)"
    elif transform_loss:
        diagnosis = "T (transform loss)"
    elif aggregation_loss:
        diagnosis = "G (aggregation loss)"
    else:
        diagnosis = "NEITHER -- A1 and A8 both close to A0; residual signal may not be recoverable via simple recomposition"

    summary["diagnosis"] = {
        "A1_vs_A0_linear_delta": a1_delta,
        "A1_vs_A0_nonlinear_delta": a1_delta_nl,
        "A8_vs_A0_linear_delta": a8_delta,
        "A8_vs_A0_nonlinear_delta": a8_delta_nl,
        "verdict": diagnosis,
    }

    RESULTS_DIR.mkdir(parents=True, exist_ok=True)
    with open(RESULTS_DIR / "discriminator_probe.json", "w") as f:
        json.dump(summary, f, indent=2)

    print(json.dumps(summary["diagnosis"], indent=2))
    print("\n--- A0-A8 linear ROC-AUC ---")
    for name in FEATURE_SETS:
        print(f"{name}: {summary['feature_sets'][name]['linear']['roc_auc']['mean']:.4f}  (nonlinear: {summary['feature_sets'][name]['nonlinear']['roc_auc']['mean']:.4f})")


if __name__ == "__main__":
    main()
