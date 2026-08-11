"""Shared constants/loaders for the chemical-space error atlas scripts."""

from pathlib import Path

import numpy as np
import pandas as pd
from sklearn.neighbors import NearestNeighbors

SCRIPT_DIR = Path(__file__).resolve().parent
RESULTS_DIR = SCRIPT_DIR / "results"
PLOTS_DIR = SCRIPT_DIR / "plots"

FEATURE_COLUMNS = [
    "heavy_atoms",
    "mol_wt",
    "ring_count",
    "ring_system_count",
    "aromatic_ring_count",
    "aromatic_atom_fraction",
    "rotatable_bonds",
    "heteroatom_count",
    "stereocenters",
    "stereocenter_density",
    "bridgeheads",
    "spiro_atoms",
    "tpsa",
    "fraction_csp3",
    "fg_count",
]

COMPONENT_COLUMNS = [
    "ring_contribution",
    "size_contribution",
    "stereo_contribution",
    "fg_contribution",
]

SEED = 0  # fixed everywhere in this directory -- see README.md's reproducibility section


def load_features():
    df = pd.read_csv(RESULTS_DIR / "features.csv")
    df["finding_codes"] = df["finding_codes"].fillna("")
    return df


def knn_local_rates(coords, is_fp, is_fn, is_hard, k=15):
    """For each point, the fraction of its k nearest neighbors (Euclidean
    in `coords` -- the caller picks the space: descriptor-PCA, UMAP, or
    YOMITOKI component space) that are FP / FN / any-error / expert-hard.
    Self is excluded. Used identically across all three spaces so the
    cross-space comparison in component_space.py is apples-to-apples.

    Returns a DataFrame with one row per input point:
    local_fp_rate, local_fn_rate, local_error_rate, local_hard_rate.
    """
    n = len(coords)
    k = min(k, n - 1)
    nn = NearestNeighbors(n_neighbors=k + 1).fit(coords)
    _, indices = nn.kneighbors(coords)
    neighbor_idx = indices[:, 1:]  # drop self (always the nearest at distance 0)

    is_fp = np.asarray(is_fp)
    is_fn = np.asarray(is_fn)
    is_hard = np.asarray(is_hard)
    is_error = is_fp | is_fn

    return pd.DataFrame(
        {
            "local_fp_rate": is_fp[neighbor_idx].mean(axis=1),
            "local_fn_rate": is_fn[neighbor_idx].mean(axis=1),
            "local_error_rate": is_error[neighbor_idx].mean(axis=1),
            "local_hard_rate": is_hard[neighbor_idx].mean(axis=1),
        }
    )


def knn_label_purity(coords, labels, k=15):
    """Fraction of each point's k nearest neighbors sharing its own label
    -- a simple, interpretable separation metric (not a formal silhouette
    score, which assumes pre-defined clusters this analysis doesn't
    have). Returns the mean purity across all points.
    """
    n = len(coords)
    k = min(k, n - 1)
    nn = NearestNeighbors(n_neighbors=k + 1).fit(coords)
    _, indices = nn.kneighbors(coords)
    neighbor_idx = indices[:, 1:]
    labels = np.asarray(labels)
    same_label = labels[neighbor_idx] == labels[:, None]
    return float(same_label.mean())
