#!/usr/bin/env python3
"""Section 8: residual-information test -- this round's most important
adoption metric. For each aggregation candidate (L1 current, L2, MAX):
rebuild F0 using that candidate's own size_contribution (ring/stereo/fg
unchanged), then re-run the SAME residual probe as
discriminator_probe.py's A8 (candidate_F0 + raw mw/rb/heteroatom burden
as three SEPARATE extra features). If the candidate has genuinely
absorbed the decomposition information the discriminator probe found,
this residual gap should shrink substantially relative to L1's own gap
(target: to <=0.02 absolute, or a >=40% reduction) -- not just "produce
a higher production AUC," which the aggregation_comparison.py numbers
already show is a much smaller effect on its own.
"""

import json
import sys
import warnings

import numpy as np
from sklearn.ensemble import HistGradientBoostingClassifier
from sklearn.linear_model import LogisticRegression
from sklearn.metrics import roc_auc_score
from sklearn.preprocessing import StandardScaler
from sklearn.utils.class_weight import compute_sample_weight

warnings.filterwarnings("ignore")

from common import AGGREGATORS, N_FOLDS, RESULTS_DIR, SEED, load_dataset, saturate

F0_BASE_COLUMNS = ["ring_contribution", "stereo_contribution", "fg_contribution"]  # size_contribution swapped in per-candidate


def fit_predict_linear(X_train, y_train, X_val):
    scaler = StandardScaler()
    X_train_s, X_val_s = scaler.fit_transform(X_train), scaler.transform(X_val)
    sw = compute_sample_weight("balanced", y_train)
    clf = LogisticRegression(penalty="l2", C=1.0, max_iter=2000, random_state=SEED)
    clf.fit(X_train_s, y_train, sample_weight=sw)
    return clf.predict_proba(X_val_s)[:, 1]


def fit_predict_nonlinear(X_train, y_train, X_val):
    sw = compute_sample_weight("balanced", y_train)
    clf = HistGradientBoostingClassifier(max_depth=3, max_iter=100, random_state=SEED)
    clf.fit(X_train, y_train, sample_weight=sw)
    return clf.predict_proba(X_val)[:, 1]


def run_feature_set(df, columns, probe_fn):
    aucs = []
    for fold_idx in range(N_FOLDS):
        train, val = df[df["fold"] != fold_idx], df[df["fold"] == fold_idx]
        X_train, y_train = train[columns].to_numpy(), train["expert_label"].to_numpy()
        X_val, y_val = val[columns].to_numpy(), val["expert_label"].to_numpy()
        score = probe_fn(X_train, y_train, X_val)
        aucs.append(float(roc_auc_score(y_val, score)))
    return aucs


def main():
    df = load_dataset()

    report = {}
    for cand_name, agg_fn in AGGREGATORS.items():
        raw = agg_fn(df["mw_burden"].to_numpy(), df["rotatable_burden"].to_numpy(), df["heteroatom_burden"].to_numpy())
        df[f"size_contribution_{cand_name}"] = saturate(raw)

        f0_cols = F0_BASE_COLUMNS + [f"size_contribution_{cand_name}"]
        f0_plus_raw_cols = f0_cols + ["mw_burden", "rotatable_burden", "heteroatom_burden"]

        f0_only_linear = run_feature_set(df, f0_cols, fit_predict_linear)
        f0_plus_raw_linear = run_feature_set(df, f0_plus_raw_cols, fit_predict_linear)
        f0_only_nonlinear = run_feature_set(df, f0_cols, fit_predict_nonlinear)
        f0_plus_raw_nonlinear = run_feature_set(df, f0_plus_raw_cols, fit_predict_nonlinear)

        gap_linear = float(np.mean(f0_plus_raw_linear) - np.mean(f0_only_linear))
        gap_nonlinear = float(np.mean(f0_plus_raw_nonlinear) - np.mean(f0_only_nonlinear))

        report[cand_name] = {
            "f0_only_linear_mean": float(np.mean(f0_only_linear)),
            "f0_plus_raw_linear_mean": float(np.mean(f0_plus_raw_linear)),
            "residual_gap_linear": gap_linear,
            "f0_only_nonlinear_mean": float(np.mean(f0_only_nonlinear)),
            "f0_plus_raw_nonlinear_mean": float(np.mean(f0_plus_raw_nonlinear)),
            "residual_gap_nonlinear": gap_nonlinear,
        }
        print(f"{cand_name}: linear gap={gap_linear:.4f}  nonlinear gap={gap_nonlinear:.4f}", file=sys.stderr)

    baseline_gap_nonlinear = report["L1_current"]["residual_gap_nonlinear"]
    for cand_name in report:
        gap = report[cand_name]["residual_gap_nonlinear"]
        reduction = 1.0 - (gap / baseline_gap_nonlinear) if baseline_gap_nonlinear else None
        report[cand_name]["reduction_vs_L1_nonlinear"] = reduction
        report[cand_name]["meets_adoption_bar"] = bool(gap <= 0.02 or (reduction is not None and reduction >= 0.40))

    RESULTS_DIR.mkdir(parents=True, exist_ok=True)
    with open(RESULTS_DIR / "residual_information_test.json", "w") as f:
        json.dump(report, f, indent=2)

    print(json.dumps(report, indent=2))


if __name__ == "__main__":
    main()
