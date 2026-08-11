#!/usr/bin/env python3
"""Chemical-Space Error Atlas, step 4 (round 22 part 12): error overlays,
FN/FP cluster quantification, and kNN local error enrichment.

This is a hypothesis generator, not a tuning tool -- nothing here reads
or writes any scoring weight/threshold/formula. See README.md.

Clustering (HDBSCAN, on the UMAP structural-similarity embedding) is
used strictly as an error-localization aid. A UMAP "island" is NOT
claimed to be an independent real chemical class -- it's just a region
this analysis can name, count, and describe consistently; the actual
claim is always about the *molecules in it*, evidenced by their own
descriptors/FindingCodes, not about the cluster boundary itself.
"""

import json
import sys
import warnings

import hdbscan
import matplotlib

matplotlib.use("Agg")
import matplotlib.pyplot as plt
import numpy as np
import pandas as pd

warnings.filterwarnings("ignore")

from common import COMPONENT_COLUMNS, FEATURE_COLUMNS, PLOTS_DIR, RESULTS_DIR, knn_local_rates, load_features

HDBSCAN_MIN_CLUSTER_SIZE = 30  # fixed, documented -- see README.md
KNN_K = 15  # matches UMAP_N_NEIGHBORS for consistency, not required to
MIN_REGION_SIZE_FOR_TOP_LIST = 20  # avoid tiny noise clusters dominating the "top enriched region" ranking


def load_merged():
    features = load_features()
    pca = pd.read_csv(RESULTS_DIR / "pca_coordinates.csv")[["id", "PC1", "PC2", "PC3"]]
    umap_df = pd.read_csv(RESULTS_DIR / "umap_coordinates.csv")[["id", "UMAP1", "UMAP2"]]
    df = features.merge(pca, on="id").merge(umap_df, on="id")
    assert len(df) == len(features), "merge dropped rows -- id mismatch between features/pca/umap"
    return df


# ---------------------------------------------------------------------------
# Plots
# ---------------------------------------------------------------------------


def _scatter_categorical(ax, x, y, categories, palette, title):
    for cat, color in palette.items():
        mask = categories == cat
        ax.scatter(x[mask], y[mask], s=4, alpha=0.5, color=color, label=cat)
    ax.set_title(title)
    ax.legend(markerscale=3, fontsize=7)


def _scatter_continuous(ax, x, y, values, title, cmap="viridis"):
    sc = ax.scatter(x, y, s=4, alpha=0.6, c=values, cmap=cmap)
    ax.set_title(title)
    plt.colorbar(sc, ax=ax, fraction=0.046)


def plot_pca_components(df, loadings_summary):
    fig, ax = plt.subplots(figsize=(7, 7))
    ax.scatter(df["PC1"], df["PC2"], s=3, alpha=0.15, color="gray")
    scale = max(df["PC1"].abs().max(), df["PC2"].abs().max()) * 0.9
    for feat in FEATURE_COLUMNS:
        lx = loadings_summary["full_loadings_per_pc"]["PC1"][feat]
        ly = loadings_summary["full_loadings_per_pc"]["PC2"][feat]
        ax.annotate(
            "",
            xy=(lx * scale, ly * scale),
            xytext=(0, 0),
            arrowprops=dict(arrowstyle="->", color="crimson", alpha=0.7),
        )
        ax.text(lx * scale * 1.1, ly * scale * 1.1, feat, fontsize=6, color="crimson")
    ax.set_xlabel("PC1")
    ax.set_ylabel("PC2")
    ax.set_title("Descriptor PCA: loadings biplot")
    fig.tight_layout()
    fig.savefig(PLOTS_DIR / "pca_components.png", dpi=150)
    plt.close(fig)


OUTCOME_PALETTE = {"TP": "#2b8cbe", "TN": "#74c476", "FP": "#e6550d", "FN": "#de2d26"}
LABEL_PALETTE = {0: "#74c476", 1: "#de2d26"}


def plot_outcomes_panel(df, x_col, y_col, out_name, title_prefix):
    fig, axes = plt.subplots(2, 2, figsize=(12, 11))
    _scatter_categorical(axes[0, 0], df[x_col], df[y_col], df["confusion"], OUTCOME_PALETTE, f"{title_prefix}: prediction outcome")
    _scatter_continuous(axes[0, 1], df[x_col], df[y_col], df["yomitoki_difficulty"], f"{title_prefix}: overall.difficulty")
    label_named = df["expert_label"].map({0: "easy", 1: "hard"})
    _scatter_categorical(
        axes[1, 0], df[x_col], df[y_col], label_named, {"easy": "#74c476", "hard": "#de2d26"}, f"{title_prefix}: expert label"
    )
    _scatter_continuous(axes[1, 1], df[x_col], df[y_col], df["agreement_fraction"], f"{title_prefix}: expert agreement", cmap="magma")
    for ax in axes.flat:
        ax.set_xlabel(x_col)
        ax.set_ylabel(y_col)
    fig.tight_layout()
    fig.savefig(PLOTS_DIR / out_name, dpi=150)
    plt.close(fig)


def plot_umap_component_overlays(df):
    for col, name in [
        ("yomitoki_difficulty", "umap_difficulty.png"),
        ("ring_contribution", "umap_ring.png"),
        ("size_contribution", "umap_size.png"),
        ("stereo_contribution", "umap_stereo.png"),
        ("fg_contribution", "umap_fg.png"),
    ]:
        fig, ax = plt.subplots(figsize=(7, 6))
        _scatter_continuous(ax, df["UMAP1"], df["UMAP2"], df[col], f"UMAP: {col}")
        ax.set_xlabel("UMAP1")
        ax.set_ylabel("UMAP2")
        fig.tight_layout()
        fig.savefig(PLOTS_DIR / name, dpi=150)
        plt.close(fig)


# ---------------------------------------------------------------------------
# Clustering (UMAP space) and per-cluster stats
# ---------------------------------------------------------------------------


def cluster_umap(df):
    coords = df[["UMAP1", "UMAP2"]].to_numpy()
    clusterer = hdbscan.HDBSCAN(min_cluster_size=HDBSCAN_MIN_CLUSTER_SIZE)
    labels = clusterer.fit_predict(coords)
    return labels  # -1 == noise/unclustered, per HDBSCAN convention


def _conditional_rate(group, error_code, class_codes):
    """error_code (e.g. "FN") / count of rows whose confusion is in
    class_codes (e.g. ("FN", "TP") -- all truly-hard rows). This is the
    class-conditional miss-rate/false-alarm-rate convention used
    throughout this project (e.g. "FN rate" = fraction of truly-hard
    molecules missed), NOT raw prevalence over every row in the group --
    conflating the two would make an all-easy region with zero hard
    molecules read as having a perfect "0% FN rate" for the wrong reason
    (no hard molecules to miss, not no misses).
    """
    denom = group["confusion"].isin(class_codes).sum()
    if denom == 0:
        return None
    return float((group["confusion"] == error_code).sum() / denom)


def cluster_summary(df, cluster_col="cluster"):
    global_fn_rate = _conditional_rate(df, "FN", ("FN", "TP"))
    global_fp_rate = _conditional_rate(df, "FP", ("FP", "TN"))
    rows = []
    for cid, group in df.groupby(cluster_col):
        if cid == -1:
            continue
        n = len(group)
        n_hard = int(group["confusion"].isin(("FN", "TP")).sum())
        n_easy = int(group["confusion"].isin(("FP", "TN")).sum())
        fn_rate = _conditional_rate(group, "FN", ("FN", "TP"))
        fp_rate = _conditional_rate(group, "FP", ("FP", "TN"))
        rows.append(
            {
                "cluster": int(cid),
                "n": int(n),
                "n_hard": n_hard,
                "n_easy": n_easy,
                "fn_rate": fn_rate,
                "fp_rate": fp_rate,
                "fn_enrichment_vs_global": float(fn_rate / global_fn_rate) if fn_rate is not None else None,
                "fp_enrichment_vs_global": float(fp_rate / global_fp_rate) if fp_rate is not None else None,
                "mean_stereocenters": float(group["stereocenters"].mean()),
                "mean_stereocenter_density": float(group["stereocenter_density"].mean()),
                "mean_ring_count": float(group["ring_count"].mean()),
                "mean_heavy_atoms": float(group["heavy_atoms"].mean()),
                "mean_fg_count": float(group["fg_count"].mean()),
                "mean_agreement_fraction": float(group["agreement_fraction"].mean()),
                "mean_expert_label": float(group["expert_label"].mean()),  # fraction labeled hard
            }
        )
    return pd.DataFrame(rows), global_fn_rate, global_fp_rate


def structural_interpretation(group, global_means):
    """Short, evidence-grounded description (mean descriptor deviations
    from the global dataset mean) -- not a speculative chemical judgment,
    just what's actually different about this region's molecules.
    """
    parts = []
    for feat, label in [
        ("heavy_atoms", "size"),
        ("ring_count", "rings"),
        ("stereocenters", "stereocenters"),
        ("aromatic_ring_count", "aromatic rings"),
        ("fg_count", "reactive FG count"),
        ("bridgeheads", "bridgeheads"),
    ]:
        local_mean = group[feat].mean()
        global_mean = global_means[feat]
        if global_mean == 0:
            continue
        ratio = local_mean / global_mean
        if ratio >= 1.5:
            parts.append(f"high {label} ({local_mean:.1f} vs global {global_mean:.1f})")
        elif ratio <= 0.67:
            parts.append(f"low {label} ({local_mean:.1f} vs global {global_mean:.1f})")
    return "; ".join(parts) if parts else "no strong descriptor deviation from the global mean"


def top_finding_codes(group, top_n=5):
    counts = {}
    for codes in group["finding_codes"]:
        for code in codes.split(";"):
            code = code.strip()
            if code:
                counts[code] = counts.get(code, 0) + 1
    ranked = sorted(counts.items(), key=lambda kv: -kv[1])[:top_n]
    return [{"code": c, "n": n} for c, n in ranked]


def region_report(df, cluster_ids, direction, global_means, top_n=10):
    """`direction`: 'fn' or 'fp'. Ranks clusters by that error type's
    class-conditional rate (miss-rate among truly-hard molecules for
    'fn', false-alarm-rate among truly-easy molecules for 'fp'), among
    clusters with both >= MIN_REGION_SIZE_FOR_TOP_LIST total molecules
    AND enough molecules of the relevant true class for the rate to be
    more than a couple of data points (same minimum, reused).
    """
    rate_col = f"{direction}_rate"
    class_count_col = "n_hard" if direction == "fn" else "n_easy"
    eligible = cluster_ids[
        (cluster_ids["n"] >= MIN_REGION_SIZE_FOR_TOP_LIST)
        & (cluster_ids[class_count_col] >= MIN_REGION_SIZE_FOR_TOP_LIST // 4)
        & cluster_ids[rate_col].notna()
    ]
    ranked = eligible.sort_values(rate_col, ascending=False).head(top_n)

    reports = []
    for _, row in ranked.iterrows():
        cid = row["cluster"]
        group = df[df["cluster"] == cid]
        centroid = group[["UMAP1", "UMAP2"]].mean().to_numpy()
        dists = np.linalg.norm(group[["UMAP1", "UMAP2"]].to_numpy() - centroid, axis=1)
        medoid_id = group.iloc[np.argmin(dists)]["id"]
        representatives = group.iloc[np.argsort(dists)[:5]]["id"].tolist()

        reports.append(
            {
                "region_id": f"{direction}_cluster_{int(cid)}",
                "n": int(row["n"]),
                "n_hard": int(row["n_hard"]),
                "n_easy": int(row["n_easy"]),
                "fp_rate": row["fp_rate"],
                "fn_rate": row["fn_rate"],
                "fp_enrichment_vs_global": row["fp_enrichment_vs_global"],
                "fn_enrichment_vs_global": row["fn_enrichment_vs_global"],
                "medoid_id": medoid_id,
                "representative_ids": representatives,
                "mean_stereocenters": row["mean_stereocenters"],
                "mean_ring_count": row["mean_ring_count"],
                "mean_heavy_atoms": row["mean_heavy_atoms"],
                "mean_fg_count": row["mean_fg_count"],
                "mean_agreement_fraction": row["mean_agreement_fraction"],
                "fraction_expert_hard": row["mean_expert_label"],
                "component_contribution_means": {c: float(group[c].mean()) for c in COMPONENT_COLUMNS},
                "dominant_finding_codes": top_finding_codes(group),
                "structural_interpretation": structural_interpretation(group, global_means),
            }
        )
    return reports


def main():
    PLOTS_DIR.mkdir(parents=True, exist_ok=True)
    df = load_merged()

    with open(RESULTS_DIR / "pca_summary.json") as f:
        pca_summary = json.load(f)

    plot_pca_components(df, pca_summary)
    plot_outcomes_panel(df, "PC1", "PC2", "pca_outcomes.png", "PCA")
    plot_outcomes_panel(df, "UMAP1", "UMAP2", "umap_outcomes.png", "UMAP")
    plot_umap_component_overlays(df)

    df["cluster"] = cluster_umap(df)
    n_clusters = df["cluster"].nunique() - (1 if (df["cluster"] == -1).any() else 0)
    n_noise = int((df["cluster"] == -1).sum())

    cluster_stats, global_fn_rate, global_fp_rate = cluster_summary(df)
    global_means = {feat: df[feat].mean() for feat in FEATURE_COLUMNS}

    top_fn = region_report(df, cluster_stats, "fn", global_means, top_n=10)
    top_fp = region_report(df, cluster_stats, "fp", global_means, top_n=10)

    # kNN local error-rate enrichment (UMAP space), ranked -- no arbitrary
    # threshold applied by default (per the round's explicit allowance);
    # `local FN rate >= global x 2` is reported as one candidate cut for
    # reference, not used to filter anything else in this script.
    knn = knn_local_rates(
        df[["UMAP1", "UMAP2"]].to_numpy(),
        is_fp=(df["confusion"] == "FP").to_numpy(),
        is_fn=(df["confusion"] == "FN").to_numpy(),
        is_hard=(df["expert_label"] == 1).to_numpy(),
        k=KNN_K,
    )
    df_knn = pd.concat([df[["id"]].reset_index(drop=True), knn], axis=1)
    df_knn.to_csv(RESULTS_DIR / "knn_local_rates_umap.csv", index=False)

    n_at_2x_fn = int((knn["local_fn_rate"] >= 2 * global_fn_rate).sum())
    n_at_2x_fp = int((knn["local_fp_rate"] >= 2 * global_fp_rate).sum())

    # Top 10 + 10 error regions -> error_regions.csv (flat, one row per region)
    region_rows = []
    for r in top_fn + top_fp:
        flat = {k: v for k, v in r.items() if k not in ("component_contribution_means", "dominant_finding_codes", "representative_ids")}
        flat["representative_ids"] = ";".join(r["representative_ids"])
        flat["dominant_finding_codes"] = ";".join(f"{d['code']}:{d['n']}" for d in r["dominant_finding_codes"])
        for c in COMPONENT_COLUMNS:
            flat[c] = r["component_contribution_means"][c]
        region_rows.append(flat)
    pd.DataFrame(region_rows).to_csv(RESULTS_DIR / "error_regions.csv", index=False)

    summary = {
        "n_molecules": int(len(df)),
        "global_fn_rate": float(global_fn_rate),
        "global_fp_rate": float(global_fp_rate),
        "hdbscan": {
            "space": "UMAP",
            "min_cluster_size": HDBSCAN_MIN_CLUSTER_SIZE,
            "n_clusters": int(n_clusters),
            "n_noise_points": n_noise,
            "note": "Clusters are an error-localization aid, not a claim of independent real chemical classes.",
        },
        "knn_enrichment": {
            "k": KNN_K,
            "space": "UMAP",
            "n_molecules_local_fn_rate_geq_2x_global": n_at_2x_fn,
            "n_molecules_local_fp_rate_geq_2x_global": n_at_2x_fp,
            "note": "2x-global is reported as one reference cut, not used as a hard filter elsewhere.",
        },
        "top_fn_regions": top_fn,
        "top_fp_regions": top_fp,
    }
    with open(RESULTS_DIR / "error_atlas_summary.json", "w") as f:
        json.dump(summary, f, indent=2)

    print(f"clusters: {n_clusters} (+{n_noise} noise points)", file=sys.stderr)
    print(f"global FN rate={global_fn_rate:.4f} FP rate={global_fp_rate:.4f}", file=sys.stderr)
    print(f"molecules with local FN rate >= 2x global: {n_at_2x_fn}", file=sys.stderr)
    print(f"molecules with local FP rate >= 2x global: {n_at_2x_fp}", file=sys.stderr)
    print("\ntop FN region:", json.dumps(top_fn[0], indent=2) if top_fn else "none", file=sys.stderr)
    print("\ntop FP region:", json.dumps(top_fp[0], indent=2) if top_fp else "none", file=sys.stderr)


if __name__ == "__main__":
    main()
