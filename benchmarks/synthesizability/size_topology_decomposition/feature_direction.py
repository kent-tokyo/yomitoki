#!/usr/bin/env python3
"""Size-Topology Information Decomposition, step 2 (round 22 part 14):
per-descriptor direction and effect-size analysis, plus a conditional
check that a descriptor's apparent hard/easy direction isn't just a
proxy for molecular size (e.g. "high TPSA -> hard" being nothing more
than "big molecules -> hard, and TPSA correlates with size").
"""

import json
import sys
import warnings

import numpy as np
import pandas as pd
from sklearn.metrics import roc_auc_score

warnings.filterwarnings("ignore")

from common import load_features

CANDIDATES = ["mol_wt", "rotatable_bonds", "fraction_csp3", "heteroatom_count", "tpsa"]


def univariate_stats(df, feature):
    hard = df.loc[df["expert_label"] == 1, feature]
    easy = df.loc[df["expert_label"] == 0, feature]
    pooled_std = np.sqrt((hard.std() ** 2 + easy.std() ** 2) / 2)
    cohens_d = (hard.mean() - easy.mean()) / pooled_std if pooled_std > 0 else None
    auc = roc_auc_score(df["expert_label"], df[feature])
    return {
        "median_hard": float(hard.median()),
        "median_easy": float(easy.median()),
        "iqr_hard": [float(hard.quantile(0.25)), float(hard.quantile(0.75))],
        "iqr_easy": [float(easy.quantile(0.25)), float(easy.quantile(0.75))],
        "cohens_d": float(cohens_d) if cohens_d is not None else None,
        "univariate_roc_auc": float(auc),
    }


def conditional_direction(df, feature, condition_feature, n_bins=4):
    """Within each quantile bin of `condition_feature`, does `feature`
    still separate hard/easy in the same direction? If the univariate
    AUC direction flips or collapses toward 0.5 once size is held
    roughly fixed, the descriptor's apparent signal is largely a size
    proxy, not independent information.
    """
    try:
        bins = pd.qcut(df[condition_feature], n_bins, duplicates="drop")
    except ValueError:
        return []
    results = []
    for b, group in df.groupby(bins, observed=True):
        if group["expert_label"].nunique() < 2 or len(group) < 20:
            results.append({"bin": str(b), "n": int(len(group)), "note": "too small / single-class, skipped"})
            continue
        auc = roc_auc_score(group["expert_label"], group[feature])
        results.append(
            {
                "bin": str(b),
                "n": int(len(group)),
                "n_hard": int((group["expert_label"] == 1).sum()),
                "n_easy": int((group["expert_label"] == 0).sum()),
                "univariate_roc_auc": float(auc),
            }
        )
    return results


def main():
    df = load_features()

    report = {}
    for feature in CANDIDATES:
        stats = univariate_stats(df, feature)
        cond_mw = conditional_direction(df, feature, "mol_wt") if feature != "mol_wt" else None
        cond_rb = conditional_direction(df, feature, "rotatable_bonds") if feature != "rotatable_bonds" else None
        report[feature] = {
            "unconditional": stats,
            "conditional_on_mol_wt_bins": cond_mw,
            "conditional_on_rotatable_bonds_bins": cond_rb,
        }

    with open("results/feature_direction.json", "w") as f:
        json.dump(report, f, indent=2)

    print(f"{'feature':<18} {'median_hard':>12} {'median_easy':>12} {'cohens_d':>10} {'univariate_AUC':>15}", file=sys.stderr)
    for feature, r in report.items():
        u = r["unconditional"]
        print(f"{feature:<18} {u['median_hard']:>12.3f} {u['median_easy']:>12.3f} {u['cohens_d']:>10.3f} {u['univariate_roc_auc']:>15.4f}", file=sys.stderr)

    print("\nheteroatom_count conditional on mol_wt bins (does it survive holding size fixed?):", file=sys.stderr)
    for b in report["heteroatom_count"]["conditional_on_mol_wt_bins"]:
        if "univariate_roc_auc" in b:
            print(f"  {b['bin']}: n={b['n']} auc={b['univariate_roc_auc']:.4f}", file=sys.stderr)
        else:
            print(f"  {b['bin']}: {b['note']}", file=sys.stderr)

    print("\nheteroatom_count conditional on rotatable_bonds bins:", file=sys.stderr)
    for b in report["heteroatom_count"]["conditional_on_rotatable_bonds_bins"]:
        if "univariate_roc_auc" in b:
            print(f"  {b['bin']}: n={b['n']} auc={b['univariate_roc_auc']:.4f}", file=sys.stderr)
        else:
            print(f"  {b['bin']}: {b['note']}", file=sys.stderr)


if __name__ == "__main__":
    main()
