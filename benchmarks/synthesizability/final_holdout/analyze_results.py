#!/usr/bin/env python3
"""Phase 6 (post-opening) + Phase 7 + Phase 8: analyze the one-shot
holdout results, using ONLY the endpoints/thresholds/strata fixed in
HOLDOUT_MANIFEST.md before any score was computed. Nothing here is a
free choice made after seeing results.
"""

import csv
import json
import sys
import warnings

import numpy as np
from scipy import stats
from sklearn.metrics import average_precision_score, balanced_accuracy_score, matthews_corrcoef, roc_auc_score

warnings.filterwarnings("ignore")

sys.path.insert(0, "../scripts")
from metrics import paired_bootstrap_diff_ci  # noqa: E402

RESULTS = "results"
BINARY_THRESHOLD = 3.0  # route_steps > 3 = hard; fixed in the manifest


def load_jsonl(path, score_field):
    out = {}
    with open(path) as f:
        for line in f:
            r = json.loads(line)
            if r["error"] is None:
                out[r["id"]] = r[score_field]
    return out


def main():
    subset = list(csv.DictReader(open(f"{RESULTS}/final_evaluation_subset_with_strata.csv")))
    ids = [f"HOLDOUT_{i:05d}" for i in range(len(subset))]
    route_steps = np.array([int(r["route_steps"]) for r in subset])
    binary_hard = (route_steps > BINARY_THRESHOLD).astype(int)

    scores = {
        "yomitoki_v0.1.1": load_jsonl(f"{RESULTS}/holdout_yomitoki_v011.jsonl", "yomitoki_difficulty"),
        "yomitoki_v0.2.0-alpha.1": load_jsonl(f"{RESULTS}/holdout_yomitoki_v02alpha1.jsonl", "yomitoki_difficulty"),
        "sascore": load_jsonl(f"{RESULTS}/holdout_sascore.jsonl", "sascore"),
        "brsascore": load_jsonl(f"{RESULTS}/holdout_brsascore.jsonl", "brsascore"),
    }
    score_arrays = {name: np.array([d[i] for i in ids]) for name, d in scores.items()}

    # --- Phase 7 primary table ---
    primary_table = {}
    for name, y_score in score_arrays.items():
        rho, rho_p = stats.spearmanr(route_steps, y_score)
        pearson_r, pearson_p = stats.pearsonr(route_steps, y_score)
        # An absolute-score midpoint cutoff is ill-defined across different
        # scales (YOMITOKI 0-1 vs SA/BR-SA 1-10) -- the binary view instead
        # classifies by SCORE RANK (top-N by score, N = n_hard from the
        # fixed route_steps>3 threshold), which is scale-invariant and
        # avoids inventing a new per-method threshold after opening.
        n_hard = int(binary_hard.sum())
        rank_order = np.argsort(-y_score)  # highest score first
        y_pred_topk = np.zeros_like(binary_hard)
        y_pred_topk[rank_order[:n_hard]] = 1
        auc = float(roc_auc_score(binary_hard, y_score))
        pr_auc = float(average_precision_score(binary_hard, y_score))
        bal_acc = float(balanced_accuracy_score(binary_hard, y_pred_topk))
        mcc = float(matthews_corrcoef(binary_hard, y_pred_topk))
        fp = int(((binary_hard == 0) & (y_pred_topk == 1)).sum())
        fn = int(((binary_hard == 1) & (y_pred_topk == 0)).sum())

        primary_table[name] = {
            "spearman_rho": float(rho), "spearman_p": float(rho_p),
            "pearson_r": float(pearson_r), "pearson_p": float(pearson_p),
            "roc_auc": auc, "pr_auc": pr_auc, "balanced_accuracy": bal_acc, "mcc": mcc,
            "fp": fp, "fn": fn,
        }
        print(f"{name}: spearman={rho:.4f} (p={rho_p:.2e})  pearson={pearson_r:.4f}  AUC={auc:.4f}  PR-AUC={pr_auc:.4f}  BalAcc={bal_acc:.4f}  MCC={mcc:.4f}  FP={fp}  FN={fn}", file=sys.stderr)

    # --- Phase 7 pairwise ---
    def spearman_metric(yt, ys):
        return stats.spearmanr(yt, ys).correlation

    pairwise = {}
    v02 = score_arrays["yomitoki_v0.2.0-alpha.1"]
    for other in ["yomitoki_v0.1.1", "sascore", "brsascore"]:
        ci = paired_bootstrap_diff_ci(route_steps, score_arrays[other], v02, metric_fn=spearman_metric)
        pairwise[f"v0.2_minus_{other}"] = ci
        print(f"v0.2 - {other}: spearman diff point={ci['point']:.4f}  95% CI [{ci['low']:.4f}, {ci['high']:.4f}]", file=sys.stderr)

    # --- Phase 8 structural strata (v0.2.0-alpha.1 only, primary candidate) ---
    strata_results = {}
    for stratum_col in ["mw_stratum", "ring_stratum", "aromatic_stratum", "stereo_stratum", "heteroatom_stratum"]:
        strata_results[stratum_col] = {}
        for level in ["low", "high"]:
            mask = np.array([r[stratum_col] == level for r in subset])
            rho, _ = stats.spearmanr(route_steps[mask], v02[mask])
            strata_results[stratum_col][level] = {"n": int(mask.sum()), "spearman_rho": float(rho)}
        print(f"{stratum_col}: low={strata_results[stratum_col]['low']}  high={strata_results[stratum_col]['high']}", file=sys.stderr)

    report = {
        "n": len(subset),
        "binary_threshold": BINARY_THRESHOLD,
        "n_hard_binary": int(binary_hard.sum()),
        "primary_table": primary_table,
        "pairwise_spearman_diff": pairwise,
        "structural_strata": strata_results,
    }
    with open(f"{RESULTS}/holdout_analysis.json", "w") as f:
        json.dump(report, f, indent=2)


if __name__ == "__main__":
    main()
