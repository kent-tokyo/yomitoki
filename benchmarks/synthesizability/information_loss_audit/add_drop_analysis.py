#!/usr/bin/env python3
"""Information-Loss Audit, step 4 (round 22 part 13): add-one-group-in
(starting from F0, current components) and leave-one-group-out
(starting from F1, raw descriptors) -- an information audit, not
production weight tuning. Same scaffold-grouped 5-fold CV, same two
probes as run_probes.py.
"""

import json
import sys
import warnings

import pandas as pd

warnings.filterwarnings("ignore")

from common import F0_COLUMNS, F2_COLUMNS, F3_COLUMNS, F4_COLUMNS, F5_COLUMNS, RESULTS_DIR
from run_probes import fit_predict_linear, fit_predict_nonlinear, paired_fold_comparison, run_feature_set, summarize_folds

# Add-one-group: F0 + each richer detail group (F2/F3/F4/F5's full
# column sets, not just F1's flattened subset -- these carry the most
# pre-compression detail available for each domain).
ADD_GROUPS = {
    "ring_detail": F2_COLUMNS,
    "stereo_detail": F3_COLUMNS,
    "size_detail": F4_COLUMNS,
    "fg_detail": F5_COLUMNS,
}

# Leave-one-group-out: groupings *within* F1's flat descriptor list.
F1_GROUPS = {
    "ring_group": ["ring_count", "ring_system_count", "aromatic_ring_count", "aromatic_atom_fraction", "bridgeheads", "spiro_atoms"],
    "stereo_group": ["stereocenters", "stereocenter_density"],
    "size_group": ["heavy_atoms", "mol_wt", "rotatable_bonds", "fraction_csp3", "heteroatom_count", "tpsa"],
    "fg_group": ["fg_count"],
}


def dedup(columns):
    seen = set()
    out = []
    for c in columns:
        if c not in seen:
            seen.add(c)
            out.append(c)
    return out


def main():
    df = pd.read_csv(RESULTS_DIR / "features_with_folds.csv")

    # --- Add-one-group-in ---
    baseline_cols = F0_COLUMNS
    baseline_linear = run_feature_set(df, baseline_cols, fit_predict_linear)
    baseline_nonlinear = run_feature_set(df, baseline_cols, fit_predict_nonlinear)

    add_results = {
        "F0_baseline": {
            "linear_auc": summarize_folds(baseline_linear, "roc_auc"),
            "nonlinear_auc": summarize_folds(baseline_nonlinear, "roc_auc"),
        }
    }
    for group_name, group_cols in ADD_GROUPS.items():
        cols = dedup(baseline_cols + group_cols)
        lin = run_feature_set(df, cols, fit_predict_linear)
        nl = run_feature_set(df, cols, fit_predict_nonlinear)
        add_results[f"F0_plus_{group_name}"] = {
            "n_features": len(cols),
            "linear_auc": summarize_folds(lin, "roc_auc"),
            "nonlinear_auc": summarize_folds(nl, "roc_auc"),
            "linear_vs_baseline": paired_fold_comparison(baseline_linear, lin, "roc_auc"),
            "nonlinear_vs_baseline": paired_fold_comparison(baseline_nonlinear, nl, "roc_auc"),
        }
        print(
            f"F0 + {group_name}: linear_auc={add_results[f'F0_plus_{group_name}']['linear_auc']['mean']:.4f} "
            f"(+{add_results[f'F0_plus_{group_name}']['linear_vs_baseline']['mean_diff']:.4f}) "
            f"nonlinear_auc={add_results[f'F0_plus_{group_name}']['nonlinear_auc']['mean']:.4f} "
            f"(+{add_results[f'F0_plus_{group_name}']['nonlinear_vs_baseline']['mean_diff']:.4f})",
            file=sys.stderr,
        )

    # --- Leave-one-group-out ---
    all_f1_cols = dedup(sum(F1_GROUPS.values(), []))
    full_linear = run_feature_set(df, all_f1_cols, fit_predict_linear)
    full_nonlinear = run_feature_set(df, all_f1_cols, fit_predict_nonlinear)

    drop_results = {
        "F1_full": {
            "n_features": len(all_f1_cols),
            "linear_auc": summarize_folds(full_linear, "roc_auc"),
            "nonlinear_auc": summarize_folds(full_nonlinear, "roc_auc"),
        }
    }
    for group_name, group_cols in F1_GROUPS.items():
        remaining = [c for c in all_f1_cols if c not in group_cols]
        lin = run_feature_set(df, remaining, fit_predict_linear)
        nl = run_feature_set(df, remaining, fit_predict_nonlinear)
        drop_results[f"F1_minus_{group_name}"] = {
            "n_features": len(remaining),
            "linear_auc": summarize_folds(lin, "roc_auc"),
            "nonlinear_auc": summarize_folds(nl, "roc_auc"),
            "linear_vs_full": paired_fold_comparison(lin, full_linear, "roc_auc"),
            "nonlinear_vs_full": paired_fold_comparison(nl, full_nonlinear, "roc_auc"),
        }
        print(
            f"F1 - {group_name}: linear_auc={drop_results[f'F1_minus_{group_name}']['linear_auc']['mean']:.4f} "
            f"(drop {-drop_results[f'F1_minus_{group_name}']['linear_vs_full']['mean_diff']:.4f}) "
            f"nonlinear_auc={drop_results[f'F1_minus_{group_name}']['nonlinear_auc']['mean']:.4f} "
            f"(drop {-drop_results[f'F1_minus_{group_name}']['nonlinear_vs_full']['mean_diff']:.4f})",
            file=sys.stderr,
        )

    with open(RESULTS_DIR / "add_drop_results.json", "w") as f:
        json.dump({"add_one_group_in": add_results, "leave_one_group_out": drop_results}, f, indent=2)
    print(f"\nwrote {RESULTS_DIR / 'add_drop_results.json'}", file=sys.stderr)


if __name__ == "__main__":
    main()
