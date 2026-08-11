#!/usr/bin/env python3
"""Chemical-Space Error Atlas, step 2: descriptor PCA (round 22 part 12).

Standardizes the interpretable descriptor set from build_features.py
(zero mean, unit variance -- required before PCA since these are on
very different scales, e.g. mol_wt ~O(100) vs. aromatic_atom_fraction
~O(1)) and projects onto its top 3 principal components. Reports not
just coordinates but which descriptors dominate each axis -- loadings
are the actual point of this step, not the scatter plot.

No scoring code touched. No TS1/TS2/TS3.
"""

import json
import sys

import numpy as np
from sklearn.decomposition import PCA
from sklearn.preprocessing import StandardScaler

from common import FEATURE_COLUMNS, RESULTS_DIR, load_features

N_COMPONENTS = 3
N_TOP_LOADINGS = 5  # descriptors reported per PC


def interpret_component(loadings_row, feature_names, top_n=N_TOP_LOADINGS):
    order = np.argsort(-np.abs(loadings_row))[:top_n]
    return [{"feature": feature_names[i], "loading": float(loadings_row[i])} for i in order]


def main():
    df = load_features()
    X = df[FEATURE_COLUMNS].to_numpy(dtype=float)

    scaler = StandardScaler()
    X_std = scaler.fit_transform(X)

    pca = PCA(n_components=N_COMPONENTS, random_state=0)
    # scikit-learn's PCA on this platform's BLAS (Accelerate, macOS ARM)
    # emits a spurious "divide by zero"/"overflow" RuntimeWarning from an
    # internal SVD numerical path -- verified harmless: output coordinates
    # contain no NaN/Inf and match a manual re-check (see README.md's
    # reproducibility section). Suppressed narrowly, only around the fit
    # call, so an unrelated real warning elsewhere in this script still
    # surfaces normally.
    with np.errstate(divide="ignore", over="ignore", invalid="ignore"):
        coords = pca.fit_transform(X_std)

    out = df[["id", "expert_label", "confusion", "yomitoki_difficulty"]].copy()
    for i in range(N_COMPONENTS):
        out[f"PC{i + 1}"] = coords[:, i]
    out_path = RESULTS_DIR / "pca_coordinates.csv"
    out.to_csv(out_path, index=False)

    top_loadings = [interpret_component(pca.components_[i], FEATURE_COLUMNS) for i in range(N_COMPONENTS)]

    summary = {
        "n_components": N_COMPONENTS,
        "n_molecules": len(df),
        "feature_columns": FEATURE_COLUMNS,
        "explained_variance_ratio": [float(v) for v in pca.explained_variance_ratio_],
        "cumulative_explained_variance_ratio": [float(v) for v in np.cumsum(pca.explained_variance_ratio_)],
        "top_loadings_per_pc": {f"PC{i + 1}": top_loadings[i] for i in range(N_COMPONENTS)},
        "full_loadings_per_pc": {
            f"PC{i + 1}": {FEATURE_COLUMNS[j]: float(pca.components_[i][j]) for j in range(len(FEATURE_COLUMNS))}
            for i in range(N_COMPONENTS)
        },
    }
    with open(RESULTS_DIR / "pca_summary.json", "w") as f:
        json.dump(summary, f, indent=2)

    print(f"wrote {out_path} ({len(out)} rows)", file=sys.stderr)
    print(f"explained variance ratio: {summary['explained_variance_ratio']}", file=sys.stderr)
    print(f"cumulative: {summary['cumulative_explained_variance_ratio']}", file=sys.stderr)
    for i in range(N_COMPONENTS):
        print(f"\nPC{i + 1} top loadings:", file=sys.stderr)
        for entry in top_loadings[i]:
            print(f"  {entry['feature']:<24} {entry['loading']:+.3f}", file=sys.stderr)


if __name__ == "__main__":
    main()
