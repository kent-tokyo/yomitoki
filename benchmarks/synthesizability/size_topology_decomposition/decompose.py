#!/usr/bin/env python3
"""Size-Topology Information Decomposition, step 1 (round 22 part 14):
separates H1 (existing-primitive compression problem -- size_topology's
current threshold/saturation transform of MW/rotatable-bond count
discards their own raw continuous information) from H2 (missing-feature
problem -- fraction_csp3/heteroatom_count/tpsa carry real NEW
information) from H3 (both).

Primary probe: L2-regularized logistic regression (same config as
information_loss_audit/run_probes.py) -- this round is about measuring
each feature's conditional information content, not maximizing
accuracy. Shallow HistGradientBoostingClassifier is run only as a
secondary confirmation, never the basis for a conclusion on its own.

Same scaffold folds as information_loss_audit (common.py loads that
round's exact features_with_folds.csv) -- every comparison here is
paired against that round's F0 result on the identical 5 folds, not
reassigned.
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

from common import F0_COLUMNS, N_FOLDS, NEW_TRIO, RAW_MW_RB, RESULTS_DIR, SEED, load_features

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
        "caveat": "n=5 folds only -- low statistical power, directional signal not a significance proof",
    }


def dedup(columns):
    seen, out = set(), []
    for c in columns:
        if c not in seen:
            seen.add(c)
            out.append(c)
    return out


# --- Feature set decomposition (section 3 + section 4 of the instruction) ---
FEATURE_SETS = {
    "S0_F0_only": F0_COLUMNS,
    "S1_raw_MW_RB": dedup(F0_COLUMNS + RAW_MW_RB),
    "S2_fsp3_only": dedup(F0_COLUMNS + ["fraction_csp3"]),
    "S3_heteroatoms_only": dedup(F0_COLUMNS + ["heteroatom_count"]),
    "S4_tpsa_only": dedup(F0_COLUMNS + ["tpsa"]),
    "S5_new_trio": dedup(F0_COLUMNS + NEW_TRIO),
    "S6_all_size_detail": dedup(F0_COLUMNS + RAW_MW_RB + NEW_TRIO),
    # Conditional-information sets (section 4): does each new descriptor
    # add independent signal *after* raw MW/RB are already present?
    "S1_plus_fsp3": dedup(F0_COLUMNS + RAW_MW_RB + ["fraction_csp3"]),
    "S1_plus_heteroatoms": dedup(F0_COLUMNS + RAW_MW_RB + ["heteroatom_count"]),
    "S1_plus_tpsa": dedup(F0_COLUMNS + RAW_MW_RB + ["tpsa"]),
}


def main():
    df = load_features()

    all_results = {}
    for name, columns in FEATURE_SETS.items():
        print(f"running {name} ({len(columns)} features: {columns})...", file=sys.stderr)
        linear = run_feature_set(df, columns, fit_predict_linear)
        nonlinear = run_feature_set(df, columns, fit_predict_nonlinear)
        all_results[name] = {"linear": linear, "nonlinear": nonlinear, "columns": columns}

    baseline_linear = all_results["S0_F0_only"]["linear"]
    baseline_nonlinear = all_results["S0_F0_only"]["nonlinear"]
    s1_linear = all_results["S1_raw_MW_RB"]["linear"]
    s1_nonlinear = all_results["S1_raw_MW_RB"]["nonlinear"]

    comparisons = {}
    for name in FEATURE_SETS:
        if name == "S0_F0_only":
            continue
        comparisons[f"{name}_vs_S0_linear_roc_auc"] = paired_fold_comparison(baseline_linear, all_results[name]["linear"], "roc_auc")
        comparisons[f"{name}_vs_S0_nonlinear_roc_auc"] = paired_fold_comparison(baseline_nonlinear, all_results[name]["nonlinear"], "roc_auc")
    # Conditional sets specifically vs S1 (not S0) -- the question is
    # marginal value *after* raw MW/RB are already included.
    for name in ["S1_plus_fsp3", "S1_plus_heteroatoms", "S1_plus_tpsa"]:
        comparisons[f"{name}_vs_S1_linear_roc_auc"] = paired_fold_comparison(s1_linear, all_results[name]["linear"], "roc_auc")
        comparisons[f"{name}_vs_S1_nonlinear_roc_auc"] = paired_fold_comparison(s1_nonlinear, all_results[name]["nonlinear"], "roc_auc")

    summary = {"feature_sets": {}, "comparisons": comparisons}
    for name, res in all_results.items():
        summary["feature_sets"][name] = {
            "columns": res["columns"],
            "linear": {m: summarize_folds(res["linear"], m) for m in ["roc_auc", "balanced_accuracy", "mcc", "hard_class_pr_auc", "easy_class_pr_auc"]},
            "nonlinear": {m: summarize_folds(res["nonlinear"], m) for m in ["roc_auc", "balanced_accuracy", "mcc", "hard_class_pr_auc", "easy_class_pr_auc"]},
        }

    with open(RESULTS_DIR / "decomposition_results.json", "w") as f:
        json.dump(summary, f, indent=2)

    print(f"\n{'set':<24} {'linear AUC':>11} {'Δ vs S0':>10} {'nonlinear AUC':>14} {'Δ vs S0':>10}", file=sys.stderr)
    for name in FEATURE_SETS:
        lin = summary["feature_sets"][name]["linear"]["roc_auc"]["mean"]
        nl = summary["feature_sets"][name]["nonlinear"]["roc_auc"]["mean"]
        d_lin = comparisons.get(f"{name}_vs_S0_linear_roc_auc")
        d_nl = comparisons.get(f"{name}_vs_S0_nonlinear_roc_auc")
        d_lin_s = f"{d_lin['mean_diff']:+.4f}" if d_lin else "  (base)"
        d_nl_s = f"{d_nl['mean_diff']:+.4f}" if d_nl else "  (base)"
        print(f"{name:<24} {lin:>11.4f} {d_lin_s:>10} {nl:>14.4f} {d_nl_s:>10}", file=sys.stderr)

    print("\nConditional on raw MW/RB (S1 + X vs S1 alone), linear AUC:", file=sys.stderr)
    for name in ["S1_plus_fsp3", "S1_plus_heteroatoms", "S1_plus_tpsa"]:
        c = comparisons[f"{name}_vs_S1_linear_roc_auc"]
        print(f"  {name}: Δ={c['mean_diff']:+.4f} p={c['paired_ttest_p_value']:.4f}", file=sys.stderr)


if __name__ == "__main__":
    main()
