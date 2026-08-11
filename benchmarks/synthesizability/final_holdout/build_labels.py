#!/usr/bin/env python3
"""Phase 2 (pre-scoring): derive per-target route-length labels from
PaRoutes' n1-routes.json BEFORE computing any YOMITOKI/comparator
scores. Route length = depth of the longest sequential chain of
reaction steps from the target to a purchasable (in_stock) precursor --
the standard retrosynthesis "number of steps" definition (convergent
branches run in parallel, so the route's real length is bounded by its
longest single branch, not the total reaction-node count).

This file's output IS the frozen label used everywhere downstream --
written once, before any score is looked at.
"""

import csv
import json
from pathlib import Path

DOWNLOADED = Path(__file__).resolve().parent / "downloaded"
RESULTS = Path(__file__).resolve().parent / "results"


def route_depth(node):
    if node["type"] == "mol":
        children = node.get("children") or []
        if not children:
            return 0
        return max(route_depth(c) for c in children)
    elif node["type"] == "reaction":
        children = node.get("children") or []
        if not children:
            return 1
        return 1 + max(route_depth(c) for c in children)
    raise ValueError(f"unknown node type: {node.get('type')}")


def main():
    with open(DOWNLOADED / "n1-routes.json") as f:
        routes = json.load(f)

    rows = []
    for route in routes:
        smiles = route["smiles"]
        depth = route_depth(route)
        rows.append({"smiles": smiles, "route_steps": depth})

    RESULTS.mkdir(parents=True, exist_ok=True)
    with open(RESULTS / "paroutes_n1_labels.csv", "w", newline="") as f:
        writer = csv.DictWriter(f, fieldnames=["smiles", "route_steps"])
        writer.writeheader()
        writer.writerows(rows)

    steps = [r["route_steps"] for r in rows]
    print(f"n routes: {len(rows)}")
    print(f"n distinct target SMILES: {len(set(r['smiles'] for r in rows))}")
    print(f"route_steps: min={min(steps)} max={max(steps)} mean={sum(steps)/len(steps):.2f}")
    from collections import Counter

    print("distribution:", sorted(Counter(steps).items()))


if __name__ == "__main__":
    main()
