#!/usr/bin/env python3
"""Sections 4-7: the central experiment. How well can route_steps be
predicted from TARGET-ONLY structure -- raw descriptors (linear +
shallow nonlinear) and Morgan fingerprint (nonlinear) -- compared to
YOMITOKI's own production score? Bemis-Murcko scaffold-grouped 5-fold
CV throughout (no random split). Evaluated on the full set, the
high/low-ring strata (from the frozen manifest's own median split), the
novel-scaffold subset, and route-depth bins.

No new hand-designed scoring rule. No hyperparameter search beyond
fixed, modest defaults -- this is a diagnostic ceiling probe, not a
production model candidate.
"""

import json
import sys
import warnings

import numpy as np
import pandas as pd
from scipy import stats
from sklearn.ensemble import HistGradientBoostingRegressor
from sklearn.linear_model import Ridge
from sklearn.metrics import mean_absolute_error
from sklearn.preprocessing import StandardScaler

warnings.filterwarnings("ignore")

sys.path.insert(0, ".")
from common import N_FOLDS, RESULTS_DIR, SEED, build_scaffold_folds, load_frozen_holdout, load_novel_scaffold_ids

RAW_DESCRIPTOR_COLUMNS = [
    "mol_wt", "heavy_atoms", "rotatable_bonds", "heteroatom_count", "tpsa", "fraction_csp3",
    "ring_count", "ring_system_count", "largest_family_size", "fused_ring_atom_count",
    "macrocycle_ring_count", "max_ring_size", "aromatic_ring_count", "aromatic_atom_fraction",
    "bridgeheads", "spiro_atoms",
    "stereocenters", "specified_stereocenters", "unspecified_stereocenters", "stereocenter_density",
    "fg_count", "fg_alert_type_count", "fg_dense_finding", "fg_max_evidence_value",
]


def rank_rmse(y_true, y_pred):
    rt = stats.rankdata(y_true)
    rp = stats.rankdata(y_pred)
    return float(np.sqrt(np.mean((rt - rp) ** 2)))


def eval_metrics(y_true, y_pred):
    rho, _ = stats.spearmanr(y_true, y_pred)
    tau, _ = stats.kendalltau(y_true, y_pred)
    return {
        "spearman_rho": float(rho),
        "kendall_tau": float(tau),
        "mae": float(mean_absolute_error(y_true, y_pred)),
        "rank_rmse": rank_rmse(y_true, y_pred),
    }


def cv_predict_linear(X, y, fold, n_folds=N_FOLDS):
    preds = np.zeros(len(y))
    for k in range(n_folds):
        train, val = fold != k, fold == k
        scaler = StandardScaler()
        X_train = scaler.fit_transform(X[train])
        X_val = scaler.transform(X[val])
        model = Ridge(alpha=1.0, random_state=SEED)
        model.fit(X_train, y[train])
        preds[val] = model.predict(X_val)
    return preds


def cv_predict_nonlinear(X, y, fold, n_folds=N_FOLDS):
    preds = np.zeros(len(y))
    for k in range(n_folds):
        train, val = fold != k, fold == k
        model = HistGradientBoostingRegressor(max_depth=4, max_iter=150, random_state=SEED)
        model.fit(X[train], y[train])
        preds[val] = model.predict(X[val])
    return preds


def main():
    holdout = load_frozen_holdout()
    holdout = build_scaffold_folds(holdout)
    desc = pd.read_csv(RESULTS_DIR / "raw_descriptors.csv")
    # desc recomputes ring_count/aromatic_ring_count/heteroatom_count
    # itself (this round's own authoritative descriptor pass) -- drop the
    # round-20 manifest's versions of those specific columns before
    # merging so the merge doesn't silently _x/_y-suffix them.
    overlap_cols = [c for c in desc.columns if c in holdout.columns and c != "id"]
    df = holdout.drop(columns=overlap_cols).merge(desc, on="id")
    fp = np.load(RESULTS_DIR / "morgan_fp.npy")
    assert fp.shape[0] == len(df)

    y = df["route_steps"].to_numpy(dtype=float)
    fold = df["fold"].to_numpy()
    X_raw = df[RAW_DESCRIPTOR_COLUMNS].to_numpy(dtype=float)

    print("running raw-linear probe (5-fold scaffold CV)...", file=sys.stderr)
    pred_raw_linear = cv_predict_linear(X_raw, y, fold)
    print("running raw-nonlinear probe...", file=sys.stderr)
    pred_raw_nonlinear = cv_predict_nonlinear(X_raw, y, fold)
    print("running Morgan fingerprint nonlinear probe...", file=sys.stderr)
    pred_morgan = cv_predict_nonlinear(fp.astype(float), y, fold)

    yomitoki = df["yomitoki_difficulty"].to_numpy()

    novel_ids = load_novel_scaffold_ids()
    is_novel = df["smiles"].isin(novel_ids).to_numpy()
    is_high_ring = (df["ring_stratum"] == "high").to_numpy()
    is_low_ring = (df["ring_stratum"] == "low").to_numpy()

    populations = {
        "full": np.ones(len(df), dtype=bool),
        "low_ring": is_low_ring,
        "high_ring": is_high_ring,
        "novel_scaffold": is_novel,
    }
    # route-depth bins: 2-3 (bulk), 4-5, 6+ (sparse tail)
    bins = pd.cut(df["route_steps"], bins=[1, 3, 5, 100], labels=["2-3", "4-5", "6+"])
    for label in ["2-3", "4-5", "6+"]:
        populations[f"route_steps_{label}"] = (bins == label).to_numpy()

    representations = {
        "yomitoki": yomitoki,
        "raw_linear": pred_raw_linear,
        "raw_nonlinear": pred_raw_nonlinear,
        "morgan_nonlinear": pred_morgan,
    }

    report = {}
    for pop_name, mask in populations.items():
        report[pop_name] = {"n": int(mask.sum())}
        for repr_name, preds in representations.items():
            if mask.sum() < 10:
                report[pop_name][repr_name] = None
                continue
            report[pop_name][repr_name] = eval_metrics(y[mask], preds[mask])

    RESULTS_DIR.mkdir(parents=True, exist_ok=True)
    with open(RESULTS_DIR / "predictability_probes.json", "w") as f:
        json.dump(report, f, indent=2)

    # Save per-molecule predictions for downstream matched-pair / synthesis analysis.
    out = df[["id", "smiles", "route_steps", "ring_stratum", "yomitoki_difficulty"]].copy()
    out["pred_raw_linear"] = pred_raw_linear
    out["pred_raw_nonlinear"] = pred_raw_nonlinear
    out["pred_morgan"] = pred_morgan
    out["is_novel_scaffold"] = is_novel
    out.to_csv(RESULTS_DIR / "predictions_per_molecule.csv", index=False)

    for pop_name in populations:
        print(f"\n--- {pop_name} (n={report[pop_name]['n']}) ---")
        for repr_name in representations:
            m = report[pop_name][repr_name]
            if m:
                print(f"  {repr_name:18s}  rho={m['spearman_rho']:.4f}  tau={m['kendall_tau']:.4f}  MAE={m['mae']:.4f}  rankRMSE={m['rank_rmse']:.1f}")


if __name__ == "__main__":
    main()
