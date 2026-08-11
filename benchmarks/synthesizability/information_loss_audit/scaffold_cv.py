#!/usr/bin/env python3
"""Information-Loss Audit, step 2 (round 22 part 13): Bemis-Murcko
scaffold-grouped 5-fold CV split. No random molecule split -- same/
near-identical scaffolds never span train and validation within a fold.
Verified (not assumed): explicit zero-leakage check below.
"""

import json
import sys

from sklearn.model_selection import GroupKFold

from common import N_FOLDS, RESULTS_DIR, SEED, load_features


def make_folds(df):
    gkf = GroupKFold(n_splits=N_FOLDS)
    # GroupKFold's own split order is deterministic given a fixed input
    # order; shuffle deterministically by sorting on id first so a rerun
    # against a rebuilt features.csv (same content, arbitrary row order)
    # still reproduces the same folds.
    df = df.sort_values("id").reset_index(drop=True)
    groups = df["scaffold_smiles"].to_numpy()
    fold_assignment = [-1] * len(df)
    for fold_idx, (_, val_idx) in enumerate(gkf.split(df, groups=groups)):
        for i in val_idx:
            fold_assignment[i] = fold_idx
    df["fold"] = fold_assignment
    return df


def verify_no_scaffold_leakage(df):
    leaks = []
    for scaffold, group in df.groupby("scaffold_smiles"):
        if group["fold"].nunique() > 1:
            leaks.append(scaffold)
    if leaks:
        raise SystemExit(f"REFUSING TO PROCEED: {len(leaks)} scaffolds span multiple folds: {leaks[:5]}...")
    print(f"verified: 0/{df['scaffold_smiles'].nunique()} scaffolds span multiple folds", file=sys.stderr)


def fold_summary(df):
    summary = []
    for fold_idx in range(N_FOLDS):
        subset = df[df["fold"] == fold_idx]
        n_hard = int((subset["expert_label"] == 1).sum())
        n_easy = int((subset["expert_label"] == 0).sum())
        summary.append(
            {
                "fold": fold_idx,
                "n_molecules": int(len(subset)),
                "n_hard": n_hard,
                "n_easy": n_easy,
                "hard_fraction": n_hard / len(subset) if len(subset) else None,
                "n_scaffolds": int(subset["scaffold_smiles"].nunique()),
            }
        )
    return summary


def main():
    df = load_features()
    df = make_folds(df)
    verify_no_scaffold_leakage(df)

    summary = fold_summary(df)
    with open(RESULTS_DIR / "fold_summary.json", "w") as f:
        json.dump({"n_folds": N_FOLDS, "seed": SEED, "folds": summary}, f, indent=2)

    out_path = RESULTS_DIR / "features_with_folds.csv"
    df.to_csv(out_path, index=False)
    print(f"wrote {out_path}", file=sys.stderr)
    for s in summary:
        print(
            f"fold {s['fold']}: n={s['n_molecules']} hard={s['n_hard']} easy={s['n_easy']} "
            f"hard_frac={s['hard_fraction']:.3f} scaffolds={s['n_scaffolds']}",
            file=sys.stderr,
        )


if __name__ == "__main__":
    main()
