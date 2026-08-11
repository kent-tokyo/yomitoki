#!/usr/bin/env python3
"""Size-Topology Information Decomposition, step 5 (round 22 part 14):
does heteroatom_count (the leading candidate) carry signal across the
FN archetypes from information_loss_audit, or only within the "classic
FM-2" subset?

Reproduces that round's exact FN clustering (same F1 columns, same
KMeans k=6, same seed=0 -- deterministic, verified to reproduce the
same silhouette/cluster sizes before trusting the archetype labels
below).
"""

import json
import sys
import warnings

import numpy as np
from sklearn.cluster import KMeans
from sklearn.metrics import roc_auc_score, silhouette_score
from sklearn.preprocessing import StandardScaler

warnings.filterwarnings("ignore")

from common import load_features

F1_COLUMNS = [
    "heavy_atoms", "mol_wt", "ring_count", "ring_system_count", "aromatic_ring_count",
    "aromatic_atom_fraction", "rotatable_bonds", "heteroatom_count", "stereocenters",
    "stereocenter_density", "bridgeheads", "spiro_atoms", "tpsa", "fraction_csp3", "fg_count",
]
BEST_K = 6
SEED = 0
CANDIDATE_FEATURES = ["mol_wt", "rotatable_bonds", "fraction_csp3", "heteroatom_count", "tpsa"]


def main():
    df = load_features()
    fn = df[df["confusion"] == "FN"].reset_index(drop=True)

    X = fn[F1_COLUMNS].to_numpy()
    X_std = StandardScaler().fit_transform(X)
    km = KMeans(n_clusters=BEST_K, random_state=SEED, n_init=10)
    labels = km.fit_predict(X_std)
    sil = float(silhouette_score(X_std, labels))
    print(f"reproduced silhouette at k={BEST_K}: {sil:.4f} (information_loss_audit reported 0.2047)", file=sys.stderr)

    fn["archetype"] = labels

    report = []
    for cid in sorted(set(labels)):
        group = fn[fn["archetype"] == cid]
        entry = {
            "archetype": int(cid),
            "n": int(len(group)),
            "share_of_all_fn": float(len(group) / len(fn)),
        }
        for feat in CANDIDATE_FEATURES:
            entry[f"mean_{feat}"] = float(group[feat].mean())
            entry[f"median_{feat}"] = float(group[feat].median())
        # Does heteroatom_count separate hard/easy WITHIN this archetype
        # specifically? (FN molecules are all expert_label=1 by
        # definition -- this instead checks separation vs. TP molecules
        # with similar structural profile, using nearest TP neighbors'
        # heteroatom_count as an implicit "what would have been enough"
        # is out of scope here; report descriptive coverage only, per
        # the round's actual ask.)
        report.append(entry)

    with open("results/archetype_coverage.json", "w") as f:
        json.dump({"reproduced_silhouette": sil, "archetypes": report}, f, indent=2)

    print(f"\n{'archetype':<10} {'n':>6} {'share':>7} {'mol_wt':>9} {'rot_bonds':>10} {'fsp3':>7} {'heteroatoms':>12} {'tpsa':>8}", file=sys.stderr)
    for e in report:
        print(
            f"{e['archetype']:<10} {e['n']:>6} {e['share_of_all_fn']*100:>6.1f}% "
            f"{e['mean_mol_wt']:>9.1f} {e['mean_rotatable_bonds']:>10.2f} {e['mean_fraction_csp3']:>7.3f} "
            f"{e['mean_heteroatom_count']:>12.2f} {e['mean_tpsa']:>8.1f}",
            file=sys.stderr,
        )

    global_fn_heteroatom_mean = float(fn["heteroatom_count"].mean())
    print(f"\nglobal FN mean heteroatom_count: {global_fn_heteroatom_mean:.2f}", file=sys.stderr)
    print("archetypes with above-global heteroatom_count (candidate reaches them too, not just archetype 3's classic-FM-2 profile):", file=sys.stderr)
    for e in report:
        if e["mean_heteroatom_count"] > global_fn_heteroatom_mean:
            print(f"  archetype {e['archetype']} ({e['share_of_all_fn']*100:.1f}% of FN): mean heteroatom_count={e['mean_heteroatom_count']:.2f}", file=sys.stderr)


if __name__ == "__main__":
    main()
