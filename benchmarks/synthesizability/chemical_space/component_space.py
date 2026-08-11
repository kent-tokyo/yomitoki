#!/usr/bin/env python3
"""Chemical-Space Error Atlas, step 6 (round 22 part 12): YOMITOKI's own
4D internal feature space (ring/size/stereo/fg contribution) -- does
expert hard/easy actually separate there? This is the most direct way
to visually/quantitatively confirm *why* MPScore's overall.difficulty
AUC sits near chance: if hard and easy genuinely overlap in the exact
4 numbers the aggregate score is built from, no threshold on their
weighted sum could ever separate them well, independent of any single
component's own individual behavior.

Also runs the cross-space comparison (component space vs. descriptor
PCA vs. Morgan UMAP) requested by the round: where does easy/hard
separate best, by several independent, complementary metrics. No
metric here is used to justify any weight/threshold change -- this is
diagnosis, not tuning.
"""

import json
import sys
import warnings

import matplotlib

matplotlib.use("Agg")
import matplotlib.pyplot as plt
import numpy as np
import pandas as pd
from sklearn.decomposition import PCA
from sklearn.metrics import silhouette_score
from sklearn.neighbors import NearestNeighbors

warnings.filterwarnings("ignore")

from common import COMPONENT_COLUMNS, PLOTS_DIR, RESULTS_DIR, knn_label_purity, knn_local_rates, load_features


def nearest_neighbor_agreement(coords, values, k=1):
    """Fraction of points whose single nearest neighbor (k=1, self
    excluded) shares the same value (label or prediction) -- distinct
    from knn_label_purity's k-neighbor average: this is specifically the
    "closest analog" comparison the round asks for.
    """
    n = len(coords)
    nn = NearestNeighbors(n_neighbors=k + 1).fit(coords)
    _, indices = nn.kneighbors(coords)
    nearest = indices[:, 1]
    values = np.asarray(values)
    return float((values[nearest] == values).mean())


def concentration_index(local_rates):
    """Coefficient of variation of a local-rate array -- a simple,
    space-comparable summary of how spatially concentrated (high CV) vs.
    pervasive/uniform (low CV) an error type is. Not a formal spatial
    statistic (e.g. Moran's I); good enough for a cross-space ranking,
    not a precision claim.
    """
    mean = local_rates.mean()
    if mean == 0:
        return 0.0
    return float(local_rates.std() / mean)


def evaluate_space(name, coords, df, k=15):
    labels = df["expert_label"].to_numpy()
    predicted = df["predicted_hard"].to_numpy()
    is_fp = (df["confusion"] == "FP").to_numpy()
    is_fn = (df["confusion"] == "FN").to_numpy()

    purity = knn_label_purity(coords, labels, k=k)
    nn_label_agreement = nearest_neighbor_agreement(coords, labels)
    nn_prediction_agreement = nearest_neighbor_agreement(coords, predicted)

    knn = knn_local_rates(coords, is_fp=is_fp, is_fn=is_fn, is_hard=(labels == 1), k=k)
    fn_concentration = concentration_index(knn["local_fn_rate"].to_numpy())
    fp_concentration = concentration_index(knn["local_fp_rate"].to_numpy())

    try:
        # Subsample for silhouette_score if large -- O(n^2) memory/time;
        # 10,543 points is borderline, cap at 5000 for speed, fixed seed.
        if len(coords) > 5000:
            rng = np.random.default_rng(0)
            idx = rng.choice(len(coords), size=5000, replace=False)
            sil = float(silhouette_score(coords[idx], labels[idx]))
        else:
            sil = float(silhouette_score(coords, labels))
    except ValueError:
        sil = None

    return {
        "space": name,
        "n_dimensions": coords.shape[1],
        "knn_label_purity_k15": purity,
        "nearest_neighbor_expert_label_agreement": nn_label_agreement,
        "nearest_neighbor_yomitoki_prediction_agreement": nn_prediction_agreement,
        "silhouette_expert_label_caution_class_imbalance_87pct_hard": sil,
        "fn_local_rate_concentration_index": fn_concentration,
        "fp_local_rate_concentration_index": fp_concentration,
        "mean_local_fn_rate": float(knn["local_fn_rate"].mean()),
        "mean_local_fp_rate": float(knn["local_fp_rate"].mean()),
    }


def plot_component_space(df):
    X = df[COMPONENT_COLUMNS].to_numpy()
    pca = PCA(n_components=2, random_state=0)
    with np.errstate(divide="ignore", over="ignore", invalid="ignore"):
        coords = pca.fit_transform(X)

    fig, axes = plt.subplots(1, 2, figsize=(13, 6))
    label_named = df["expert_label"].map({0: "easy", 1: "hard"})
    for cat, color in [("easy", "#74c476"), ("hard", "#de2d26")]:
        mask = label_named == cat
        axes[0].scatter(coords[mask, 0], coords[mask, 1], s=4, alpha=0.4, color=color, label=cat)
    axes[0].set_title("YOMITOKI component space (PCA of 4 contributions): expert label")
    axes[0].set_xlabel("PC1")
    axes[0].set_ylabel("PC2")
    axes[0].legend(markerscale=3)

    sc = axes[1].scatter(coords[:, 0], coords[:, 1], s=4, alpha=0.5, c=df["yomitoki_difficulty"], cmap="viridis")
    axes[1].set_title("YOMITOKI component space: overall.difficulty")
    axes[1].set_xlabel("PC1")
    axes[1].set_ylabel("PC2")
    plt.colorbar(sc, ax=axes[1], fraction=0.046)

    fig.tight_layout()
    fig.savefig(PLOTS_DIR / "component_space_pca.png", dpi=150)
    plt.close(fig)

    return {
        "explained_variance_ratio": [float(v) for v in pca.explained_variance_ratio_],
        "loadings": {f"PC{i + 1}": {COMPONENT_COLUMNS[j]: float(pca.components_[i][j]) for j in range(4)} for i in range(2)},
    }


def main():
    df = load_features()
    pca_coords = pd.read_csv(RESULTS_DIR / "pca_coordinates.csv")[["id", "PC1", "PC2", "PC3"]]
    umap_coords = pd.read_csv(RESULTS_DIR / "umap_coordinates.csv")[["id", "UMAP1", "UMAP2"]]
    df = df.merge(pca_coords, on="id").merge(umap_coords, on="id")

    component_pca_summary = plot_component_space(df)

    spaces = {
        "yomitoki_component_space_raw_4d": df[["ring_contribution", "size_contribution", "stereo_contribution", "fg_contribution"]].to_numpy(),
        "descriptor_pca_3d": df[["PC1", "PC2", "PC3"]].to_numpy(),
        "morgan_umap_2d": df[["UMAP1", "UMAP2"]].to_numpy(),
    }

    results = [evaluate_space(name, coords, df) for name, coords in spaces.items()]

    summary = {
        "n_molecules": int(len(df)),
        "component_space_pca_for_visualization": component_pca_summary,
        "cross_space_comparison": results,
    }
    with open(RESULTS_DIR / "component_space_summary.json", "w") as f:
        json.dump(summary, f, indent=2)

    print(f"component space PCA explained variance (2D, just for the plot): {component_pca_summary['explained_variance_ratio']}", file=sys.stderr)
    print(f"\n{'space':<32} {'purity':>8} {'nn_label':>9} {'nn_pred':>9} {'silhouette':>11} {'fn_conc':>8} {'fp_conc':>8}", file=sys.stderr)
    for r in results:
        sil = r["silhouette_expert_label_caution_class_imbalance_87pct_hard"]
        sil_str = f"{sil:.4f}" if sil is not None else "n/a"
        print(
            f"{r['space']:<32} {r['knn_label_purity_k15']:>8.4f} {r['nearest_neighbor_expert_label_agreement']:>9.4f} "
            f"{r['nearest_neighbor_yomitoki_prediction_agreement']:>9.4f} {sil_str:>11} "
            f"{r['fn_local_rate_concentration_index']:>8.4f} {r['fp_local_rate_concentration_index']:>8.4f}",
            file=sys.stderr,
        )


if __name__ == "__main__":
    main()
