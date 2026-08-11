#!/usr/bin/env python3
"""Section 9: route metadata audit. Extracts route-ARCHITECTURE
features from n1-routes.json (total reaction count, longest path
[=route_steps itself], branching factor, number of leaves/stock
precursors, average precursor "distance from target") -- features of
the ROUTE, not the target molecule -- to see how much of route_steps'
variance is explained by route architecture that a target-only score
could never see. Semantic-diagnostic only; no route information is
proposed for production YOMITOKI (see Renkin boundary, section 15).
"""

import json
import sys

import numpy as np
import pandas as pd

sys.path.insert(0, ".")
from common import RESULTS_DIR, load_frozen_holdout

HOLDOUT_DIR = RESULTS_DIR.parent.parent / "final_holdout"


def route_stats(node, depth=0):
    """Returns (n_reaction_nodes_total, longest_path, n_leaf_mols,
    max_depth_seen) via one recursive pass."""
    if node["type"] == "mol":
        children = node.get("children") or []
        if not children:
            return {"n_reactions": 0, "longest_path": 0, "n_leaves": 1}
        # a mol node has at most one reaction child
        return route_stats(children[0], depth)
    elif node["type"] == "reaction":
        children = node.get("children") or []
        if not children:
            return {"n_reactions": 1, "longest_path": 1, "n_leaves": 1}
        sub = [route_stats(c, depth + 1) for c in children]
        return {
            "n_reactions": 1 + sum(s["n_reactions"] for s in sub),
            "longest_path": 1 + max(s["longest_path"] for s in sub),
            "n_leaves": sum(s["n_leaves"] for s in sub),
        }
    raise ValueError(node.get("type"))


def main():
    holdout = load_frozen_holdout()
    frozen_smiles = set(holdout["smiles"])

    with open(HOLDOUT_DIR / "downloaded" / "n1-routes.json") as f:
        routes = json.load(f)

    rows = []
    for route in routes:
        if route["smiles"] not in frozen_smiles:
            continue  # excluded by the leakage audit -- stay consistent with the frozen subset
        stats = route_stats(route)
        n_reactions = stats["n_reactions"]
        longest_path = stats["longest_path"]
        n_leaves = stats["n_leaves"]
        # branching factor: how much wider the route is than a purely
        # linear chain of the same length (1.0 = perfectly linear/no
        # convergence; higher = more convergent, more independent
        # starting materials brought together).
        branching_factor = n_reactions / longest_path if longest_path else 1.0
        rows.append(
            {
                "smiles": route["smiles"],
                "route_steps": longest_path,
                "n_reactions_total": n_reactions,
                "n_leaf_precursors": n_leaves,
                "branching_factor": branching_factor,
            }
        )

    df = pd.DataFrame(rows)
    merged = holdout.merge(df, on="smiles", suffixes=("", "_route"))
    assert (merged["route_steps"] == merged["route_steps_route"]).all(), "route_steps mismatch -- route parsing bug"

    from scipy import stats as sstats

    report = {"n": len(merged)}
    for col in ["n_reactions_total", "n_leaf_precursors", "branching_factor"]:
        rho, p = sstats.spearmanr(merged["route_steps"], merged[col])
        report[f"route_steps_vs_{col}"] = {"spearman_rho": float(rho), "p": float(p)}

    # How much of route_steps variance is "explained" by architecture
    # features that are near-tautological with route_steps itself
    # (n_reactions_total, branching_factor) vs. how these correlate with
    # TARGET structure (from raw_descriptors.csv, if already built).
    report["distributions"] = {
        "n_reactions_total": merged["n_reactions_total"].describe().to_dict(),
        "n_leaf_precursors": merged["n_leaf_precursors"].describe().to_dict(),
        "branching_factor": merged["branching_factor"].describe().to_dict(),
    }

    RESULTS_DIR.mkdir(parents=True, exist_ok=True)
    merged[["smiles", "route_steps", "n_reactions_total", "n_leaf_precursors", "branching_factor"]].to_csv(RESULTS_DIR / "route_architecture.csv", index=False)
    with open(RESULTS_DIR / "route_metadata_audit.json", "w") as f:
        json.dump(report, f, indent=2)

    print(json.dumps({k: v for k, v in report.items() if k != "distributions"}, indent=2))
    print("n_reactions_total describe:", report["distributions"]["n_reactions_total"])
    print("branching_factor describe:", report["distributions"]["branching_factor"])


if __name__ == "__main__":
    main()
