#!/usr/bin/env python3
"""Section 7: high-ring collapse decomposition. Within the high-ring
subset specifically, does route_steps correlate with any FINER
structural distinction (ring-system count, aromaticity, largest fused
family, fused-vs-independent rings, bridge/spiro, MW, RB, heteroatoms)?
If nothing inside high-ring predicts route_steps either, that's
evidence the collapse isn't "wrong sub-feature" but a real ceiling for
this population. If something DOES predict within high-ring, section 6
Case C-vs-D distinction sharpens: high-ring isn't uniformly unpredictable,
some finer structural axis still carries signal there.
"""

import json
import sys

import pandas as pd
from scipy import stats

sys.path.insert(0, ".")
from common import RESULTS_DIR, load_frozen_holdout

CANDIDATE_COLUMNS = [
    "ring_count", "ring_system_count", "largest_family_size", "fused_ring_atom_count",
    "macrocycle_ring_count", "max_ring_size", "aromatic_ring_count", "aromatic_atom_fraction",
    "bridgeheads", "spiro_atoms", "mol_wt", "rotatable_bonds", "heteroatom_count", "tpsa",
]


def main():
    holdout = load_frozen_holdout()
    desc = pd.read_csv(RESULTS_DIR / "raw_descriptors.csv")
    overlap_cols = [c for c in desc.columns if c in holdout.columns and c != "id"]
    df = holdout.drop(columns=overlap_cols).merge(desc, on="id")

    high_ring = df[df["ring_stratum"] == "high"].copy()
    low_ring = df[df["ring_stratum"] == "low"].copy()

    report = {"n_high_ring": len(high_ring), "n_low_ring": len(low_ring), "within_high_ring": {}, "within_low_ring": {}}
    for col in CANDIDATE_COLUMNS:
        rho_h, p_h = stats.spearmanr(high_ring["route_steps"], high_ring[col])
        rho_l, p_l = stats.spearmanr(low_ring["route_steps"], low_ring[col])
        report["within_high_ring"][col] = {"spearman_rho": float(rho_h), "p": float(p_h)}
        report["within_low_ring"][col] = {"spearman_rho": float(rho_l), "p": float(p_l)}

    # also: does route_steps within high-ring correlate with YOMITOKI's
    # own difficulty at all, even weakly, once we're already inside the
    # collapsed regime?
    rho_y, p_y = stats.spearmanr(high_ring["route_steps"], high_ring["yomitoki_difficulty"])
    report["within_high_ring_yomitoki"] = {"spearman_rho": float(rho_y), "p": float(p_y)}

    with open(RESULTS_DIR / "high_ring_decomposition.json", "w") as f:
        json.dump(report, f, indent=2)

    print(f"high-ring subset (n={len(high_ring)}): finer-descriptor correlations with route_steps")
    for col, v in sorted(report["within_high_ring"].items(), key=lambda kv: -abs(kv[1]["spearman_rho"])):
        print(f"  {col:24s}  rho={v['spearman_rho']:+.4f}  p={v['p']:.2e}")
    print(f"\nfor comparison, low-ring subset (n={len(low_ring)}):")
    for col, v in sorted(report["within_low_ring"].items(), key=lambda kv: -abs(kv[1]["spearman_rho"])):
        print(f"  {col:24s}  rho={v['spearman_rho']:+.4f}  p={v['p']:.2e}")


if __name__ == "__main__":
    main()
